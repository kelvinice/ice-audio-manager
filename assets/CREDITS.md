# Desktop icons

The Ice snowflake and Pandora record with a faceted diamond were drawn for this project in SVG.
Each has an optical variant on a 16 px grid. The Windows ICOs include nine sizes from 16 to 256 px.

The sources are in `src/`. Run `node assets/build/icons.cjs` with sharp available to Node to
rebuild the PNG and ICO files in each Tauri project's icons directory. Sharp is a development
tool used only for rasterizing these assets.
