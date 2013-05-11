extern mod png;

use zlib_decompress = png::zlib::decompress::decompress;

fn main() {
  let test_path = os::self_exe_path().or(Some(os::getcwd())).unwrap();

  io::println(fmt!("testing files from %s", test_path.to_str()));

  for os::list_dir_path(&test_path).each |&subpath| {
    do task::spawn_supervised {
      test_file(subpath);
    }
  }
}

fn test_file(path: &Path) {
  match path.filetype() {
    Some(~".zlib") => {
      let reference = &path.with_filetype(&"out");
      if os::path_exists(reference) {
        let compressed = io::read_whole_file(path).unwrap();
        let expected = io::read_whole_file(reference).unwrap();
        match zlib_decompress(compressed) {
          Ok(actual) => {
            if actual == expected {
              io::print(fmt!("%s: ok\n", path.filename().unwrap()));
            } else {
              io::print(fmt!("%s: does not match\n", path.filename().unwrap()));
            }
          },
          Err(err) => {
            io::print(fmt!("%s: error: %s\n",
              path.filename().unwrap(), err.to_str()));
          }
        }
      } else {
        io::print(fmt!("%s: no reference output\n", path.filename().unwrap()));
      }
    },
    _ => { }
  }
}
