# Block textures

Texture aliases and their image files are listed in `textures.json`. Each image
must be a 32×32 PNG. The build script packs the listed images into the atlas and
generates the alias-to-atlas lookup; tile numbers are assigned automatically.

To add a texture, add its PNG here and add an entry such as
`{ "name": "granite", "file": "granite.png" }` to the manifest. Then use the
`granite` alias in the block definition's `texture_top`, `texture_side`, or
`texture_bottom` field. An alias may point to an existing image when blocks
share a texture.
