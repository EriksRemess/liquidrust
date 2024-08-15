# liquidrust

A simple Rust application for displaying liquid temperature for the Corsair H115i RGB PRO XT AIO cooler.

## Usage

```zsh
cargo install --git https://github.com/EriksRemess/liquidrust.git liquidrust

liquidrust

liquidrust --json

liquidrust --json  | jq -r '"\(.liquid.value)\(.liquid.units)"'

liquidrust --color 00FF00
```


### Related

https://github.com/liquidctl/liquidctl/blob/main/liquidctl/driver/hydro_platinum.py

https://gitlab.com/CalcProgrammer1/OpenRGB/-/tree/master/Controllers/CorsairHydroPlatinumController
