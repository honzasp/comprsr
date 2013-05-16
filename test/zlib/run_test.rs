extern mod comprsr;

use zlib_decompress = comprsr::zlib::decompress::decompress;

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
        let compressed = io::file_reader(path).unwrap();
        let expected = io::read_whole_file(reference).unwrap();

        let mut opt_err = None;
        let actual = do io::with_bytes_writer |writer| {
          opt_err = zlib_decompress(compressed, writer);
        };

        match opt_err {
          None => {
            if actual == expected {
              io::print(fmt!("%s: ok\n", path.filename().unwrap()));
            } else {
              io::print(fmt!("%s: does not match (.err file generated)\n",
                path.filename().unwrap()));
              let err_path = &path.with_filetype(&"err");
              io::file_writer(err_path, &[io::Create, io::Truncate])
                .unwrap().write(actual);
            }
          },
          Some(err) => {
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
