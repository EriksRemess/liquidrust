# liquidrust

A simple Rust application for displaying information and setting RGB colors for the Corsair H115i RGB PRO XT AIO.

## Usage

```zsh
cargo install --git https://github.com/EriksRemess/liquidrust.git liquidrust

liquidrust

liquidrust --json

liquidrust --json  | jq -r '"\(.liquid.value)\(.liquid.units)"'

liquidrust --color 00FF00
```

how leds are mapped on the device:

```rust
{
    { NA,  11,  12,  13,  NA },
    { 10,  NA,  1,   NA,  14 },
    { 9,   0,   NA,  2,   15 },
    { 8,   NA,  3,   NA,   4 },
    { NA,  7,   6,   5,   NA }
}
```



### Related

https://github.com/liquidctl/liquidctl/blob/main/liquidctl/driver/hydro_platinum.py

https://gitlab.com/CalcProgrammer1/OpenRGB/-/tree/master/Controllers/CorsairHydroPlatinumController
