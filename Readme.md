# PSD_LIB
Library written in Rust for working with Adobe Photoshop® `.psd` files.

Package includes a library and three binaries:

* ### psd_decompose binary

  `psd_decompose` allows to decompose psd file into chunks of objects which it store in the `decomposed_objects` directory and `{$file}.psd.decomposed` text file next to original file.

  The reason for this binary is an ability to decompose multiple files in the same directory and store them as chunks, therefore reducing the total size because of shared chunks. In my case size size reduce is from 10% up to 50%.

  Usage:

  ```bash
  $: psd_decompose [...file.psd > 1]
  
  $: psd_decompose --restore [--prefix=string] [--postfix=string] [...file.psd.decomposed > 1]
     --prefix:  prepend string to restored filename
     --postfix: append string to restored filename before extension
     
  $: psd_decompose --sha [...file > 1]
     compute sha256 hash of given prospective restored files or ordinary files. Usefull to check that restore will be correct.
     
  $: psd_decompose --remove [...file.decomposed > 1]
     removes decomposed index file and rebuilds (actually gather all the hashes from other files in the directory and removes hashes which are orphaned) decomposed_opjects directory.
     
  $: psd_decompose --cleanup
     perform cleanup of "decomposed_objects" directory, which consists of populating unique index of every hash of every .decomposed file and removing every hash which doesn't said index contains.
  ```

* ### psd_diff

  Tool for creating, applying and combining psd diff files. Based on [bin_diff](https://github.com/Reeywhaar/bin_diff) library. Usage:

  ```
  $: psd_diff measure|create|apply|combine [...args]

  $: psd_diff measure [--in-bytes] file_a.psd file_b.psd
      output size in bytes instead of human readable version

  $: psd_diff create file_a.psd file_b.psd file_a_b.psd.diff
      output file can be substituted with "-", what means output to stdout

  $: psd_diff apply file_a.psd [...file_a_b.psd.diff>1] file_b.psd
      output file can be substituted with "-", what means output to stdout

  $: psd_diff combine [...a.psd.diff>2] output.psd.diff
      output file can be substituted with "-", what means output to stdout
  ```

  Also setting environment `PSDDIFF_VERBOSE` to `true` will force command to print elapsed time

* ### psd_analyzer

  Tool which shows binary blocks representation of the file in text format. Usage:

  ```
  $: psd_analyzer [--fullpath] [--flat] [--with-size] [--with-hash] file.psd [> analysis.txt]
      --fullpath: show full path
      --flat: don't indent blocks
      --with-size: show block size in bytes
      --with-hash: append hash to each block
  ```

* ### psd_lines

  Tool for comparing multiple files. Usage:

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

Library documentation available with `cargo doc --no-deps --open` command.

## Diff Format
Diff format specification available [here](./psd_diff_spec.md)

## PSD Specification
PSD format specification that was used to create this library available [here](./psd_spec.md)

Will be gratefull for any corrections on said spec.