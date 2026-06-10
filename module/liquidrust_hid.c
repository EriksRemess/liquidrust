// SPDX-License-Identifier: GPL-2.0
/*
 * HID driver for reading Corsair H115i RGB PRO XT telemetry.
 *
 * This mirrors the Rust app's info path: send command 0xff in a 64-byte HID
 * output report, wait for a 64-byte response, validate CRC8, then parse the
 * same offsets used by src/info.rs.
 */

#include <linux/completion.h>
#include <linux/device.h>
#include <linux/err.h>
#include <linux/hid.h>
#include <linux/hwmon.h>
#include <linux/jiffies.h>
#include <linux/module.h>
#include <linux/mutex.h>
#include <linux/random.h>
#include <linux/slab.h>
#include <linux/spinlock.h>
#include <linux/sysfs.h>

#define LIQUIDRUST_VENDOR_ID 0x1b1c
#define LIQUIDRUST_PRODUCT_ID 0x0c21

#define LIQUIDRUST_REPORT_ID 0x3f
#define LIQUIDRUST_REPORT_LEN 64
#define LIQUIDRUST_CRC_OFFSET 63
#define LIQUIDRUST_STATUS_CMD 0xff
#define LIQUIDRUST_CACHE_MS 1000
#define LIQUIDRUST_TIMEOUT_MS 1000

struct liquidrust_device {
	struct hid_device *hdev;
	struct device *hwmon_dev;

	/*
	 * sysfs/hwmon reads can happen concurrently. io_lock keeps only one
	 * command in flight, while response_lock protects data touched by the
	 * HID raw_event callback.
	 */
	struct mutex io_lock;
	spinlock_t response_lock;
	struct completion response_ready;

	/* USB transfer buffers must not live on the stack. */
	u8 request[LIQUIDRUST_REPORT_LEN];
	u8 response[LIQUIDRUST_REPORT_LEN];
	bool response_pending;
	bool response_valid;
	unsigned long last_update;
};

static u8 liquidrust_crc8(const u8 *data, size_t len)
{
	u8 crc = 0;
	size_t i;
	int bit;

	for (i = 0; i < len; i++) {
		crc ^= data[i];
		for (bit = 0; bit < 8; bit++) {
			if (crc & 0x80)
				crc = (crc << 1) ^ 0x07;
			else
				crc <<= 1;
		}
	}

	return crc;
}

/* The Rust app uses a random high 5-bit nonce for most commands. */
static u8 liquidrust_random_byte(void)
{
	return (u8)(((get_random_u32() % 31) + 1) << 3);
}

static u16 liquidrust_u16le(const u8 *report, size_t offset)
{
	return (u16)report[offset] | ((u16)report[offset + 1] << 8);
}

static u32 liquidrust_temp_millicelsius(const u8 *report)
{
	return ((u32)report[8] * 1000) + (((u32)report[7] * 1000) / 255);
}

static u32 liquidrust_millipercent(u8 value)
{
	return ((u32)value * 100000) / 255;
}

static u8 liquidrust_hwmon_pwm(u8 value)
{
	/*
	 * lm-sensors 3.6 displays hwmon PWM values as half-percent units.
	 * Convert the device's 0..255 duty byte to 0..200 so sensors reports
	 * a percentage matching the exact *_millipercent attributes.
	 */
	return (u8)(((u32)value * 200 + 127) / 255);
}

static const char *liquidrust_pump_mode(u8 mode)
{
	switch (mode) {
	case 0:
		return "Quiet";
	case 1:
		return "Balanced";
	case 2:
		return "Extreme";
	default:
		return "Balanced";
	}
}

static void liquidrust_format_milli(char *buf, size_t size, u32 milli)
{
	scnprintf(buf, size, "%u.%03u", milli / 1000, milli % 1000);
}

static void liquidrust_build_status_request(u8 *request)
{
	/*
	 * Same request as src/info.rs:
	 *   byte 0  = HID report ID
	 *   byte 1  = command 0xff
	 *   byte 63 = CRC8 over bytes 1..62
	 */
	memset(request, 0, LIQUIDRUST_REPORT_LEN);
	request[0] = LIQUIDRUST_REPORT_ID;
	request[1] = liquidrust_random_byte() | LIQUIDRUST_STATUS_CMD;
	request[LIQUIDRUST_CRC_OFFSET] =
		liquidrust_crc8(&request[1], LIQUIDRUST_CRC_OFFSET - 1);
}

static int liquidrust_send_status_request(struct liquidrust_device *ldev,
					  u8 *request)
{
	int ret;

	/*
	 * Prefer the interrupt/output-report path. Some USB HID stacks do not
	 * implement it for this device, so fall back to SET_REPORT over control.
	 */
	ret = hid_hw_output_report(ldev->hdev, request, LIQUIDRUST_REPORT_LEN);
	if (ret != -ENOSYS && ret != -EOPNOTSUPP)
		return ret;

	return hid_hw_raw_request(ldev->hdev, LIQUIDRUST_REPORT_ID, request,
				  LIQUIDRUST_REPORT_LEN, HID_OUTPUT_REPORT,
				  HID_REQ_SET_REPORT);
}

static int liquidrust_refresh(struct liquidrust_device *ldev, u8 *report)
{
	unsigned long flags;
	unsigned long timeout;
	int ret;

	mutex_lock(&ldev->io_lock);

	/* Reuse a fresh response briefly so one sensors run does not spam USB. */
	spin_lock_irqsave(&ldev->response_lock, flags);
	if (ldev->response_valid &&
	    time_before(jiffies,
			ldev->last_update + msecs_to_jiffies(LIQUIDRUST_CACHE_MS))) {
		memcpy(report, ldev->response, LIQUIDRUST_REPORT_LEN);
		spin_unlock_irqrestore(&ldev->response_lock, flags);
		mutex_unlock(&ldev->io_lock);
		return 0;
	}

	reinit_completion(&ldev->response_ready);
	ldev->response_pending = true;
	spin_unlock_irqrestore(&ldev->response_lock, flags);

	/*
	 * The send is synchronous, but the device response arrives later as an
	 * input report delivered to liquidrust_raw_event().
	 */
	liquidrust_build_status_request(ldev->request);

	ret = liquidrust_send_status_request(ldev, ldev->request);
	if (ret < 0) {
		spin_lock_irqsave(&ldev->response_lock, flags);
		ldev->response_pending = false;
		spin_unlock_irqrestore(&ldev->response_lock, flags);
		mutex_unlock(&ldev->io_lock);
		return ret;
	}

	timeout = wait_for_completion_timeout(
		&ldev->response_ready, msecs_to_jiffies(LIQUIDRUST_TIMEOUT_MS));
	if (!timeout) {
		spin_lock_irqsave(&ldev->response_lock, flags);
		ldev->response_pending = false;
		spin_unlock_irqrestore(&ldev->response_lock, flags);
		mutex_unlock(&ldev->io_lock);
		return -ETIMEDOUT;
	}

	spin_lock_irqsave(&ldev->response_lock, flags);
	memcpy(report, ldev->response, LIQUIDRUST_REPORT_LEN);
	spin_unlock_irqrestore(&ldev->response_lock, flags);

	mutex_unlock(&ldev->io_lock);
	return 0;
}

static int liquidrust_get_report(struct device *dev, u8 *report)
{
	struct hid_device *hdev = to_hid_device(dev);
	struct liquidrust_device *ldev = hid_get_drvdata(hdev);

	if (!ldev)
		return -ENODEV;

	return liquidrust_refresh(ldev, report);
}

static int liquidrust_raw_event(struct hid_device *hdev, struct hid_report *hid_report,
				u8 *data, int size)
{
	struct liquidrust_device *ldev = hid_get_drvdata(hdev);
	unsigned long flags;

	if (!ldev || size < LIQUIDRUST_REPORT_LEN)
		return 0;

	if (data[LIQUIDRUST_CRC_OFFSET] !=
	    liquidrust_crc8(&data[1], LIQUIDRUST_CRC_OFFSET - 1))
		return 0;

	/* Wake the sysfs/hwmon reader waiting in liquidrust_refresh(). */
	spin_lock_irqsave(&ldev->response_lock, flags);
	if (ldev->response_pending) {
		memcpy(ldev->response, data, LIQUIDRUST_REPORT_LEN);
		ldev->response_valid = true;
		ldev->response_pending = false;
		ldev->last_update = jiffies;
		complete(&ldev->response_ready);
	}
	spin_unlock_irqrestore(&ldev->response_lock, flags);

	return 0;
}

static ssize_t measurements_show(struct device *dev, struct device_attribute *attr,
				 char *buf)
{
	u8 report[LIQUIDRUST_REPORT_LEN];
	char temp[16];
	char pump_speed[16];
	char fan1_duty[16];
	char fan2_duty[16];
	int ret;

	ret = liquidrust_get_report(dev, report);
	if (ret)
		return ret;

	liquidrust_format_milli(temp, sizeof(temp),
				liquidrust_temp_millicelsius(report));
	liquidrust_format_milli(pump_speed, sizeof(pump_speed),
				liquidrust_millipercent(report[28]));
	liquidrust_format_milli(fan1_duty, sizeof(fan1_duty),
				liquidrust_millipercent(report[14]));
	liquidrust_format_milli(fan2_duty, sizeof(fan2_duty),
				liquidrust_millipercent(report[21]));

	return sysfs_emit(
		buf,
		"Firmware: %u.%u.%u\n"
		"Liquid temperature: %s C\n"
		"Pump speed: %s %%\n"
		"Pump mode: %s\n"
		"Fan 1 speed: %u RPM\n"
		"Fan 1 duty: %s %%\n"
		"Fan 2 speed: %u RPM\n"
		"Fan 2 duty: %s %%\n",
		report[2] >> 4, report[2] & 0x0f, report[3], temp, pump_speed,
		liquidrust_pump_mode(report[24]), liquidrust_u16le(report, 15),
		fan1_duty, liquidrust_u16le(report, 22), fan2_duty);
}
static DEVICE_ATTR_RO(measurements);

static ssize_t firmware_show(struct device *dev, struct device_attribute *attr,
			     char *buf)
{
	u8 report[LIQUIDRUST_REPORT_LEN];
	int ret;

	ret = liquidrust_get_report(dev, report);
	if (ret)
		return ret;

	return sysfs_emit(buf, "%u.%u.%u\n", report[2] >> 4,
			  report[2] & 0x0f, report[3]);
}
static DEVICE_ATTR_RO(firmware);

static ssize_t liquid_temperature_millicelsius_show(
	struct device *dev, struct device_attribute *attr, char *buf)
{
	u8 report[LIQUIDRUST_REPORT_LEN];
	int ret;

	ret = liquidrust_get_report(dev, report);
	if (ret)
		return ret;

	return sysfs_emit(buf, "%u\n", liquidrust_temp_millicelsius(report));
}
static DEVICE_ATTR_RO(liquid_temperature_millicelsius);

static ssize_t pump_speed_millipercent_show(struct device *dev,
					    struct device_attribute *attr,
					    char *buf)
{
	u8 report[LIQUIDRUST_REPORT_LEN];
	int ret;

	ret = liquidrust_get_report(dev, report);
	if (ret)
		return ret;

	return sysfs_emit(buf, "%u\n", liquidrust_millipercent(report[28]));
}
static DEVICE_ATTR_RO(pump_speed_millipercent);

static ssize_t pump_mode_show(struct device *dev, struct device_attribute *attr,
			      char *buf)
{
	u8 report[LIQUIDRUST_REPORT_LEN];
	int ret;

	ret = liquidrust_get_report(dev, report);
	if (ret)
		return ret;

	return sysfs_emit(buf, "%s\n", liquidrust_pump_mode(report[24]));
}
static DEVICE_ATTR_RO(pump_mode);

static ssize_t pump_mode_raw_show(struct device *dev, struct device_attribute *attr,
				  char *buf)
{
	u8 report[LIQUIDRUST_REPORT_LEN];
	int ret;

	ret = liquidrust_get_report(dev, report);
	if (ret)
		return ret;

	return sysfs_emit(buf, "%u\n", report[24]);
}
static DEVICE_ATTR_RO(pump_mode_raw);

static ssize_t fan1_speed_rpm_show(struct device *dev,
				   struct device_attribute *attr, char *buf)
{
	u8 report[LIQUIDRUST_REPORT_LEN];
	int ret;

	ret = liquidrust_get_report(dev, report);
	if (ret)
		return ret;

	return sysfs_emit(buf, "%u\n", liquidrust_u16le(report, 15));
}
static DEVICE_ATTR_RO(fan1_speed_rpm);

static ssize_t fan1_duty_millipercent_show(struct device *dev,
					   struct device_attribute *attr,
					   char *buf)
{
	u8 report[LIQUIDRUST_REPORT_LEN];
	int ret;

	ret = liquidrust_get_report(dev, report);
	if (ret)
		return ret;

	return sysfs_emit(buf, "%u\n", liquidrust_millipercent(report[14]));
}
static DEVICE_ATTR_RO(fan1_duty_millipercent);

static ssize_t fan2_speed_rpm_show(struct device *dev,
				   struct device_attribute *attr, char *buf)
{
	u8 report[LIQUIDRUST_REPORT_LEN];
	int ret;

	ret = liquidrust_get_report(dev, report);
	if (ret)
		return ret;

	return sysfs_emit(buf, "%u\n", liquidrust_u16le(report, 22));
}
static DEVICE_ATTR_RO(fan2_speed_rpm);

static ssize_t fan2_duty_millipercent_show(struct device *dev,
					   struct device_attribute *attr,
					   char *buf)
{
	u8 report[LIQUIDRUST_REPORT_LEN];
	int ret;

	ret = liquidrust_get_report(dev, report);
	if (ret)
		return ret;

	return sysfs_emit(buf, "%u\n", liquidrust_millipercent(report[21]));
}
static DEVICE_ATTR_RO(fan2_duty_millipercent);

static ssize_t raw_report_show(struct device *dev, struct device_attribute *attr,
			       char *buf)
{
	u8 report[LIQUIDRUST_REPORT_LEN];
	int ret;
	int len = 0;
	int i;

	ret = liquidrust_get_report(dev, report);
	if (ret)
		return ret;

	for (i = 0; i < LIQUIDRUST_REPORT_LEN; i++)
		len += sysfs_emit_at(buf, len, "%02x%s", report[i],
				     i == LIQUIDRUST_REPORT_LEN - 1 ? "\n" : " ");

	return len;
}
static DEVICE_ATTR_RO(raw_report);

/*
 * Standard hwmon callbacks used by lm-sensors. These expose the values that
 * have natural hwmon equivalents: temperature, fan RPM, and PWM-like duty.
 */
static int liquidrust_hwmon_read(struct device *dev,
				 enum hwmon_sensor_types type, u32 attr,
				 int channel, long *val)
{
	struct liquidrust_device *ldev = dev_get_drvdata(dev);
	u8 report[LIQUIDRUST_REPORT_LEN];
	int ret;

	if (!ldev)
		return -ENODEV;

	ret = liquidrust_refresh(ldev, report);
	if (ret)
		return ret;

	switch (type) {
	case hwmon_temp:
		if (attr == hwmon_temp_input && channel == 0) {
			*val = liquidrust_temp_millicelsius(report);
			return 0;
		}
		break;
	case hwmon_fan:
		if (attr == hwmon_fan_input) {
			if (channel == 0) {
				*val = liquidrust_u16le(report, 15);
				return 0;
			}
			if (channel == 1) {
				*val = liquidrust_u16le(report, 22);
				return 0;
			}
		}
		break;
	case hwmon_pwm:
		if (attr == hwmon_pwm_input) {
			if (channel == 0) {
				*val = liquidrust_hwmon_pwm(report[14]);
				return 0;
			}
			if (channel == 1) {
				*val = liquidrust_hwmon_pwm(report[21]);
				return 0;
			}
			if (channel == 2) {
				*val = liquidrust_hwmon_pwm(report[28]);
				return 0;
			}
		}
		if (attr == hwmon_pwm_enable &&
		    (channel == 0 || channel == 1 || channel == 2)) {
			*val = 1;
			return 0;
		}
		break;
	default:
		break;
	}

	return -EOPNOTSUPP;
}

/* Human-readable labels for the hwmon channels that support labels. */
static int liquidrust_hwmon_read_string(struct device *dev,
					enum hwmon_sensor_types type, u32 attr,
					int channel, const char **str)
{
	switch (type) {
	case hwmon_temp:
		if (attr == hwmon_temp_label && channel == 0) {
			*str = "Liquid temperature";
			return 0;
		}
		break;
	case hwmon_fan:
		if (attr == hwmon_fan_label) {
			if (channel == 0) {
				*str = "Fan 1";
				return 0;
			}
			if (channel == 1) {
				*str = "Fan 2";
				return 0;
			}
		}
		break;
	default:
		break;
	}

	return -EOPNOTSUPP;
}

static const struct hwmon_ops liquidrust_hwmon_ops = {
	.visible = 0444,
	.read = liquidrust_hwmon_read,
	.read_string = liquidrust_hwmon_read_string,
};

static const struct hwmon_channel_info * const liquidrust_hwmon_info[] = {
	HWMON_CHANNEL_INFO(temp, HWMON_T_INPUT | HWMON_T_LABEL),
	HWMON_CHANNEL_INFO(fan, HWMON_F_INPUT | HWMON_F_LABEL,
			   HWMON_F_INPUT | HWMON_F_LABEL),
	HWMON_CHANNEL_INFO(pwm, HWMON_PWM_INPUT | HWMON_PWM_ENABLE,
			   HWMON_PWM_INPUT | HWMON_PWM_ENABLE,
			   HWMON_PWM_INPUT | HWMON_PWM_ENABLE),
	NULL,
};

static const struct hwmon_chip_info liquidrust_hwmon_chip_info = {
	.ops = &liquidrust_hwmon_ops,
	.info = liquidrust_hwmon_info,
};

/*
 * Pump mode has no standard lm-sensors feature type, so expose it as an extra
 * hwmon sysfs file next to the channels sensors already understands.
 */
static ssize_t liquidrust_hwmon_pump_mode_show(struct device *dev,
					       struct device_attribute *attr,
					       char *buf)
{
	struct liquidrust_device *ldev = dev_get_drvdata(dev);
	u8 report[LIQUIDRUST_REPORT_LEN];
	int ret;

	if (!ldev)
		return -ENODEV;

	ret = liquidrust_refresh(ldev, report);
	if (ret)
		return ret;

	return sysfs_emit(buf, "%s\n", liquidrust_pump_mode(report[24]));
}

static ssize_t liquidrust_hwmon_pump_mode_raw_show(struct device *dev,
						   struct device_attribute *attr,
						   char *buf)
{
	struct liquidrust_device *ldev = dev_get_drvdata(dev);
	u8 report[LIQUIDRUST_REPORT_LEN];
	int ret;

	if (!ldev)
		return -ENODEV;

	ret = liquidrust_refresh(ldev, report);
	if (ret)
		return ret;

	return sysfs_emit(buf, "%u\n", report[24]);
}

static ssize_t liquidrust_hwmon_pump_speed_millipercent_show(
	struct device *dev, struct device_attribute *attr, char *buf)
{
	struct liquidrust_device *ldev = dev_get_drvdata(dev);
	u8 report[LIQUIDRUST_REPORT_LEN];
	int ret;

	if (!ldev)
		return -ENODEV;

	ret = liquidrust_refresh(ldev, report);
	if (ret)
		return ret;

	return sysfs_emit(buf, "%u\n", liquidrust_millipercent(report[28]));
}

static struct device_attribute dev_attr_hwmon_pump_mode =
	__ATTR(pump_mode, 0444, liquidrust_hwmon_pump_mode_show, NULL);
static struct device_attribute dev_attr_hwmon_pump_mode_raw =
	__ATTR(pump_mode_raw, 0444, liquidrust_hwmon_pump_mode_raw_show, NULL);
static struct device_attribute dev_attr_hwmon_pump_speed_millipercent =
	__ATTR(pump_speed_millipercent, 0444,
	       liquidrust_hwmon_pump_speed_millipercent_show, NULL);

static struct attribute *liquidrust_hwmon_extra_attrs[] = {
	&dev_attr_hwmon_pump_mode.attr,
	&dev_attr_hwmon_pump_mode_raw.attr,
	&dev_attr_hwmon_pump_speed_millipercent.attr,
	NULL,
};

static const struct attribute_group liquidrust_hwmon_extra_group = {
	.attrs = liquidrust_hwmon_extra_attrs,
};

static const struct attribute_group *liquidrust_hwmon_extra_groups[] = {
	&liquidrust_hwmon_extra_group,
	NULL,
};

static struct attribute *liquidrust_attrs[] = {
	&dev_attr_measurements.attr,
	&dev_attr_firmware.attr,
	&dev_attr_liquid_temperature_millicelsius.attr,
	&dev_attr_pump_speed_millipercent.attr,
	&dev_attr_pump_mode.attr,
	&dev_attr_pump_mode_raw.attr,
	&dev_attr_fan1_speed_rpm.attr,
	&dev_attr_fan1_duty_millipercent.attr,
	&dev_attr_fan2_speed_rpm.attr,
	&dev_attr_fan2_duty_millipercent.attr,
	&dev_attr_raw_report.attr,
	NULL,
};

static const struct attribute_group liquidrust_attr_group = {
	.attrs = liquidrust_attrs,
};

/* Called when the HID core binds this driver to the Corsair device. */
static int liquidrust_probe(struct hid_device *hdev,
			    const struct hid_device_id *id)
{
	struct liquidrust_device *ldev;
	int ret;

	ldev = devm_kzalloc(&hdev->dev, sizeof(*ldev), GFP_KERNEL);
	if (!ldev)
		return -ENOMEM;

	ldev->hdev = hdev;
	mutex_init(&ldev->io_lock);
	spin_lock_init(&ldev->response_lock);
	init_completion(&ldev->response_ready);
	hid_set_drvdata(hdev, ldev);

	ret = hid_parse(hdev);
	if (ret)
		return ret;

	ret = hid_hw_start(hdev, HID_CONNECT_DRIVER | HID_CONNECT_HIDRAW);
	if (ret)
		return ret;

	/* Keep the interrupt input pipe open so raw_event can receive replies. */
	ret = hid_hw_open(hdev);
	if (ret)
		goto err_stop;

	ret = sysfs_create_group(&hdev->dev.kobj, &liquidrust_attr_group);
	if (ret)
		goto err_close;

	ldev->hwmon_dev = devm_hwmon_device_register_with_info(
		&hdev->dev, "liquidrust", ldev, &liquidrust_hwmon_chip_info,
		liquidrust_hwmon_extra_groups);
	if (IS_ERR(ldev->hwmon_dev)) {
		ret = PTR_ERR(ldev->hwmon_dev);
		goto err_remove_sysfs;
	}

	hid_info(hdev, "liquidrust telemetry driver loaded\n");
	return 0;

err_remove_sysfs:
	sysfs_remove_group(&hdev->dev.kobj, &liquidrust_attr_group);
err_close:
	hid_hw_close(hdev);
err_stop:
	hid_hw_stop(hdev);
	return ret;
}

/* Undo probe-time registration when the device or module goes away. */
static void liquidrust_remove(struct hid_device *hdev)
{
	sysfs_remove_group(&hdev->dev.kobj, &liquidrust_attr_group);
	hid_hw_close(hdev);
	hid_hw_stop(hdev);
}

static const struct hid_device_id liquidrust_devices[] = {
	{ HID_USB_DEVICE(LIQUIDRUST_VENDOR_ID, LIQUIDRUST_PRODUCT_ID) },
	{ }
};
MODULE_DEVICE_TABLE(hid, liquidrust_devices);

static struct hid_driver liquidrust_driver = {
	.name = "liquidrust_hid",
	.id_table = liquidrust_devices,
	.probe = liquidrust_probe,
	.remove = liquidrust_remove,
	.raw_event = liquidrust_raw_event,
};
module_hid_driver(liquidrust_driver);

MODULE_AUTHOR("Eriks Remess");
MODULE_DESCRIPTION("Corsair H115i RGB PRO XT telemetry HID driver");
MODULE_LICENSE("GPL");
