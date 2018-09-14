extern crate bin_diff;
extern crate psd_lib;

mod proxy_file;

use proxy_file::ProxyFile;
use psd_lib::psd_file::PSDFile;
use std::env::args;
use std::path::{Path, PathBuf};
use std::process::exit;

fn merge<T, U>(path: T, output: U) -> Result<(), String>
where
	T: AsRef<Path>,
	U: AsRef<Path>,
{
	let mut output = ProxyFile::from(PathBuf::from(output.as_ref()));
	let mut psd = PSDFile::from(path);
	psd.write_composite(&mut output)?;
	output.end()?;
	Ok(())
}

fn print_usage() {
	let usage_str = "\
usage:
$: psd_merge $input_file $output_file
   $output_file can be substituted with \"-\" which means output to stdout\
";
	println!("{}", usage_str);
}

fn main() {
	let mut args = args().skip(1);
	if args.len() < 2 {
		print_usage();
		exit(1);
	};

	let path = args.next().unwrap();
	let output = args.next().unwrap();
	let res = merge(path, output);
	if res.is_err() {
		eprintln!("{}", res.unwrap_err().to_string());
		exit(1);
	};
}
