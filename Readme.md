# PSD_LIB
Library written in Rust for working with Adobe Photoshop® `.psd` files.

Package includes a library and three binaries:

* psd_diff

  tool for creating, applying and combining psd diff files. Based on [bin_diff](https://github.com/Reeywhaar/bin_diff) library. Usage:

  ```
  $: psd_diff create|apply|combine [...args]
  $: psd_diff create file_a.psd file_b.psd file_a_b.psd.diff
  $: psd_diff apply file_a.psd [...file_a_b.psd.diff>1] file_b.psd
  $: psd_diff combine [...psd.diff>2] output.psd.diff
  ```

* psd_analyzer

  tool which shows binary blocks representation of the file in text format. Usage:

  ```
  $: psd_analyzer [--fullpath] [--flat] [--with-size] [--with-hash] file.psd [> analysis.txt]
      --fullpath: show full path
      --flat: don't indent blocks
      --with-size: show block size in bytes
      --with-hash: append hash to each block
  ```

* psd_lines

  tool for comparing multiple files. Usage:

  ```
  $: psd_lines [--truncate] [...file.psd>1] > lines.txt
      --truncate: truncate block label
  ```

## Installation & Usage
Rust must be installed on your system.

```
git clone https://github.com/Reeywhaar/psd_lib
cd psd_lib
cargo build --release
./target/release/psd_diff create ./test_data/a_a.psd ./test_data/a_b.psd ./test_data/a_a_a_b.psd.diff
```

Library documentation available with `cargo doc --no-deps open` command.

## Diff Format
Diff format specification available [here](./psd_diff_spec.md)

## PSD Specification
PSD format specification that was used to create this library available [here](./psd_spec.md)

Will be gratefull for any corrections on said spec.