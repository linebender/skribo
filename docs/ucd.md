# Notes on generation of UCD data

The data in this crate was generated from Unicode 12.0. To re-spin:

Fetch the UCD data and unpack.

```
curl -LO https://www.unicode.org/Public/zipped/12.0.0/UCD.zip
mkdir ucd
cd ucd
unzip ../UCD.zip
```

Update list of scripts known to HarfBuzz. We derived the list from [harfbuzz_sys/src/lib.rs](https://github.com/servo/rust-harfbuzz/blob/master/harfbuzz-sys/src/lib.rs) and using a text editor, pasting the result as the `hb_scripts` variable in gen_tables.py. Note that four scripts are present in Unicode 12.0 but not in harfbuzz_sys 0.3.1 ('Elymaic', 'Nandinagari', 'Nyiakeng_Puachue_Hmong', 'Wancho'). Consider updating the script to parse the Rust source file (though this would mean another download).

Run gen_tables.py. Note also that when running on Windows, you'll probably want to strip the CR
from the CRLF line endings.

```
python gen_tables.py ucd > src/tables.py
cargo fmt
```

We considered using [ucd-generate] but it did not get the data we needed in the correct form. For future work, consider migrating to that tool. Also consider trie lookups rather than binary searches, but one reason we did go for binary search is the relatively compact data.

[ucd-generate]: https://github.com/BurntSushi/ucd-generate
