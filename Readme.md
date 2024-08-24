# liquidrust

A simple Rust application for displaying information and setting RGB colors for the Corsair H115i RGB PRO XT AIO.

### Installation

```bash
cargo install liquidrust
```


## Usage

```bash
liquidrust --info # display information about the device, also default output

liquidrust --json # display information about the device in json format

liquidrust --json | jq -r '"\(.liquid_temperature.value)\(.liquid_temperature.units)"' # display liquid temperature

liquidrust --color 00FF00 # set green color for all leds

liquidrust -a FF0000 -b 00FF00 # gradient from red to green

liquidrust --rainbow # rainbow effect

liquidrust --pump balanced # set pump mode (possible modes: quiet, balanced, extreme)
```

### Bugs?

Report them here: https://github.com/EriksRemess/liquidrust/issues

### Related

How leds are mapped on the device:

```rust
{
    { NA,  11,  12,  13,  NA },
    { 10,  NA,  1,   NA,  14 },
    { 9,   0,   NA,  2,   15 },
    { 8,   NA,  3,   NA,   4 },
    { NA,  7,   6,   5,   NA }
}
```

https://github.com/liquidctl/liquidctl/blob/main/liquidctl/driver/hydro_platinum.py

https://gitlab.com/CalcProgrammer1/OpenRGB/-/tree/master/Controllers/CorsairHydroPlatinumController
