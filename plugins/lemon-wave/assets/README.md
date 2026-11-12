# Plasma factory wavetables

`plasma-wavetables.pwt` contains all 22 factory wavetables from the Gridline
Plasma source vendored in ML Plasma. Each of the 576 frames retains the
factory generator's 2,048 float samples. Gridline Plasma is by lolitemaultes
and is licensed GPL-3.0-only. See `../../ml-plasma/vendor/gridline/LICENSE`.

To regenerate on Windows, use an x64 Visual Studio Developer Command Prompt.
From the source root, build the exporter into your scratch directory with
`/std:c++17 /EHsc /O2`, compiling these two inputs together:

- `plugins/lemon-wave/tools/export-plasma-wavetables.cpp`
- `plugins/ml-plasma/vendor/gridline/src/packs/plasma/plasma_wt_dsp.cpp`

Run the resulting executable with the desired output path for
`plasma-wavetables.pwt`. The exporter invokes Plasma's actual
`factoryTableNames()` and `factoryTable()` functions. The pack stores a
`PWT1` header, frame size and table count, then each table's name length,
frame count, UTF-8 name and IEEE float samples. All numeric fields are
little-endian. Run the Wave library tests after updating the pack.
