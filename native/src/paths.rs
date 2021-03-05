use anyhow::Result;
use dunce::canonicalize;
use git2::Repository;
use log::warn;
use path_slash::{PathBufExt, PathExt};
use pathdiff::diff_paths;
use std::{
  env, fmt,
  path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct CrossPath {
  pub buf: PathBuf,
}

impl CrossPath {
  pub fn new<S: AsRef<str> + ?Sized>(s: &S) -> CrossPath {
    CrossPath {
      buf: PathBuf::from_slash(s),
    }
  }

  pub fn home() -> CrossPath {
    CrossPath::new(dirs::home_dir().unwrap().to_str().unwrap())
  }

  pub fn join<P: AsRef<Path>>(&self, path: P) -> CrossPath {
    CrossPath {
      buf: self.buf.join(path),
    }
  }

  pub fn parent(&self) -> CrossPath {
    CrossPath {
      buf: self.buf.parent().unwrap_or(Path::new("/")).to_path_buf(),
    }
  }

  pub fn canonicalize(&mut self) {
    if let Ok(b) = canonicalize(self.buf.as_path()) {
      self.buf = b;
    } else {
      warn!("Failed to canonicalize path");
    }
  }

  pub fn branch(&self) -> Option<String> {
    match Repository::discover(&self.buf.as_path()) {
      Ok(repo) => {
        let r = repo.head();
        match r {
          Ok(r) => match r.shorthand() {
            Some(s) => Some(s.to_string()),
            None => None,
          },
          _ => None,
        }
      }
      _ => None,
    }
  }

  pub fn pretty_path(&self) -> String {
    if let Ok(repo) = Repository::discover(&self.buf.as_path()) {
      let repo_name = repo
        .path()
        .parent()
        .unwrap_or(Path::new("/"))
        .parent()
        .unwrap_or(Path::new("/"));
      match diff_paths(&self.buf.as_path(), repo_name) {
        Some(diff) => diff.to_str().unwrap().to_string(),
        _ => self.to_string(),
      }
    } else {
      self
        .to_string()
        .replacen(&CrossPath::home().to_string(), "~", 1)
    }
  }
}

impl fmt::Display for CrossPath {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.buf.to_slash_lossy())
  }
}

/// Path to Termy's root directory
///
/// Used for:
///  - external resources
///  - setting.json
///  - termy.db
///  - ...
///
/// | Environment| Dev | Production |
/// | --- | --- | --- |
/// | MacOS| `termy/electron/native` | Production |
/// | Linux| Dev | Production |
/// | Windows| Dev | Production |
///
/// Path is different during build time, running Termy in development mode and running in production
pub fn root_path() -> Result<PathBuf> {
  let path = if cfg!(debug_assertions) {
    env::current_dir()?
  } else {
    // https://www.electron.build/configuration/contents.html#filesetto
    env::current_exe()?
  };
  Ok(path)
}

#[allow(dead_code)]
pub fn test_dir() -> Result<PathBuf> {
  Ok(root_path()?.join("test"))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  #[cfg(target_os = "windows")]
  fn converts_backslash_to_forward_slash() {
    let cross_path = CrossPath::new(r"C:\foo\bar.txt");
    assert_eq!("C:/foo/bar.txt", &(cross_path.to_string()));
  }

  #[test]
  fn home_directory() {
    let home = CrossPath::home().to_string();
    assert!(!home.contains(r"\"));

    #[cfg(target_os = "linux")]
    assert!(home.contains("/home"));

    #[cfg(not(target_os = "linux"))]
    assert!(home.contains("Users"));
  }

  #[test]
  fn current_branch() {
    let cwd = CrossPath::new(std::env::current_dir().unwrap().to_str().unwrap());
    assert!(cwd.branch().is_some());
  }

  #[test]
  fn pretty() {
    let cwd = CrossPath::new(std::env::current_dir().unwrap().to_str().unwrap());
    assert_eq!(cwd.pretty_path().to_lowercase(), "termy/native".to_string());
    assert_eq!(
      cwd.parent().pretty_path().to_lowercase(),
      "termy".to_string()
    );

    assert_eq!(CrossPath::home().pretty_path(), "~".to_string());

    assert_eq!(
      CrossPath::home().join("dev").pretty_path(),
      "~/dev".to_string()
    );

    #[cfg(target_os = "linux")]
    assert_eq!(CrossPath::new("/users").pretty_path(), "/users".to_string());

    #[cfg(not(target_os = "linux"))]
    assert_eq!(CrossPath::new("/Users").pretty_path(), "/Users".to_string());
  }
}
