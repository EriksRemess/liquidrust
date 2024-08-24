# liquidrust

A simple Rust application for displaying information and setting RGB colors for the Corsair H115i RGB PRO XT AIO.

## Usage

```bash
cargo install liquidrust

liquidrust --info # display information about the device, also default output

liquidrust --json # display information about the device in json format

liquidrust --json | jq -r '"\(.liquid.value)\(.liquid.units)"' # display liquid temperature

liquidrust --color 00FF00 # set green color for all leds

liquidrust -a FF0000 -b 00FF00 # gradient from red to green

liquidrust --rainbow # rainbow effect
```

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
