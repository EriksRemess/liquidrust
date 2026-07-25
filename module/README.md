# liquidrust kernel module

Out-of-tree HID driver for the Corsair H115i RGB PRO XT device used by the
Rust application in this repository.

The module sends the same `0xff` HID command as `src/info.rs`, validates the
CRC8 byte, and exposes the parsed response through sysfs.

## Build

```bash
make -C module
```

## DKMS install

Install DKMS and matching kernel headers first if needed:

```bash
sudo apt install dkms linux-headers-$(uname -r)
```

On this machine DKMS already signs modules on build, as shown by the NVIDIA
DKMS module. In that case no module-specific signing hook is needed.

If you install on a machine where DKMS is not already configured for Secure
Boot signing, configure DKMS to use your key and certificate:

```bash
sudo install -d /etc/dkms/framework.conf.d
sudo tee /etc/dkms/framework.conf.d/liquidrust-signing.conf >/dev/null <<'EOF'
sign_file="/lib/modules/$kernelver/build/scripts/sign-file"
mok_signing_key="/home/eriks/Development/kernel/keys/signing.key"
mok_certificate="/home/eriks/Development/kernel/keys/signing.crt"
EOF
```

Install this module as DKMS source:

```bash
sudo rm -rf /usr/src/liquidrust_hid-0.2.3
sudo install -d /usr/src/liquidrust_hid-0.2.3
sudo cp -a module/. /usr/src/liquidrust_hid-0.2.3/

sudo dkms add -m liquidrust_hid -v 0.2.3
sudo dkms build -m liquidrust_hid -v 0.2.3
sudo dkms install -m liquidrust_hid -v 0.2.3
```

Load the installed module:

```bash
sudo rmmod liquidrust_hid 2>/dev/null || true
sudo modprobe liquidrust_hid
```

Check DKMS status:

```bash
dkms status liquidrust_hid
modinfo liquidrust_hid | grep -E 'filename|signer|sig_key|sig_hashalgo'
```

## Load

```bash
sudo insmod module/liquidrust_hid.ko
```

If the device is already bound to `hid-generic`, unbind it and bind it to this
driver:

```bash
DEV=$(basename /sys/bus/hid/devices/*1B1C:0C21*)
echo -n "$DEV" | sudo tee /sys/bus/hid/drivers/hid-generic/unbind
echo -n "$DEV" | sudo tee /sys/bus/hid/drivers/liquidrust_hid/bind
```

## Read measurements

```bash
DEV=/sys/bus/hid/devices/$(basename /sys/bus/hid/devices/*1B1C:0C21*)
cat "$DEV/measurements"
cat "$DEV/liquid_temperature_millicelsius"
cat "$DEV/fan1_speed_rpm"
```

Available attributes:

- `measurements`
- `firmware`
- `liquid_temperature_millicelsius`
- `pump_speed_millipercent`
- `pump_mode`
- `pump_mode_raw`
- `fan1_speed_rpm`
- `fan1_duty_millipercent`
- `fan2_speed_rpm`
- `fan2_duty_millipercent`
- `raw_report`

The individual numeric files are machine-friendly sysfs values. Percent values
are scaled by 1000, so `37500` means `37.500%`.

## lm-sensors

The module also registers a `liquidrust` hwmon device. After loading and
binding the driver, `sensors` should show:

- `temp1` / `Liquid temperature`
- `fan1` / `Fan 1`
- `fan2` / `Fan 2`
- `pwm1`
- `pwm2`
- `pwm3` / pump speed

The `pwm1`, `pwm2`, and `pwm3` hwmon values are scaled for lm-sensors
percentage display. Use `fan1_duty_millipercent`, `fan2_duty_millipercent`,
and `pump_speed_millipercent` for exact device values.

Pump mode is not a standard lm-sensors feature, so it is exposed as extra hwmon
sysfs files:

```bash
cat /sys/class/hwmon/hwmon*/pump_mode 2>/dev/null
cat /sys/class/hwmon/hwmon*/pump_mode_raw 2>/dev/null
```

## Unload

```bash
sudo rmmod liquidrust_hid
```

Remove the DKMS installation with:

```bash
sudo dkms remove -m liquidrust_hid -v 0.2.3 --all
sudo rm -rf /usr/src/liquidrust_hid-0.2.3
```
