use colored::*;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;

fn folder_name_from_path(path: &str) -> String {
    let (_, package_name) = path.rsplit_once('/').unwrap();
    package_name.to_string()
}

fn get_base_config(package_name: &str) -> String {
    format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2023"

[dependencies]
bazel-toolchain = "0.8.0"
google-test = "1.13.0""#,
        package_name,
    )
}

fn get_main() -> String {
    r#"#include <ctime>
#include <string>
#include <iostream>

std::string get_greet(const std::string& who) {
  return "Hello " + who;
}

void print_localtime() {
  std::time_t result = std::time(nullptr);
  std::cout << std::asctime(std::localtime(&result));
}

int main(int argc, char** argv) {
  std::string who = "world";
  if (argc > 1) {
    who = argv[1];
  }
  std::cout << get_greet(who) << std::endl;
  print_localtime();
  return 0;
}"#
    .to_string()
}

fn get_test() -> String {
    r#"#include <gtest/gtest.h>

// Demonstrate some basic assertions.
TEST(HelloTest, BasicAssertions) {
  // Expect two strings not to be equal.
  EXPECT_STRNE("hello", "world");
  // Expect equality.
  EXPECT_EQ(7 * 6, 42);
}"#
    .to_string()
}

pub fn run(path: &str) -> Result<(), String> {
    if Path::new("Buddy.toml").exists() {
        Err("`buddy init` cannot be run on existing Buddy packages".to_string())
    } else {
        let folder_path = PathBuf::from(path);
        let path = fs::canonicalize(&folder_path).unwrap();

        if !folder_path.is_dir() {
            fs::create_dir_all(&path).unwrap();
        }

        let package_name = folder_name_from_path(path.to_str().unwrap());

        let mut file = File::create(folder_path.join("Buddy.toml")).unwrap();
        file.write_all(get_base_config(&package_name).as_bytes())
            .unwrap();

        if !folder_path.join("WORKSPACE").exists() {
            File::create(folder_path.join("WORKSPACE")).unwrap();

            if !folder_path.join("src").is_dir() {
                fs::create_dir_all(folder_path.join("src")).unwrap();
            }

            if !folder_path.join("src").join("main.cc").is_file() {
                let mut file = File::create(folder_path.join("src").join("main.cc")).unwrap();

                file.write_all(get_main().as_bytes()).unwrap();
            }

            if !folder_path.join("test").is_dir() {
                fs::create_dir_all(folder_path.join("test")).unwrap();
            }

            if !folder_path.join("test").join("test_main.cc").is_file() {
                let mut file = File::create(folder_path.join("test").join("test_main.cc")).unwrap();

                file.write_all(get_test().as_bytes()).unwrap();
            }
        }

        println!(
            "    {} binary (application) `{}` package",
            "Created".green(),
            path.to_str().unwrap()
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_run_on_empty_project() {
        let tmp_dir = tempfile::tempdir().unwrap();

        // Create empty folder
        let path = tmp_dir.path().join("test_project");
        fs::create_dir_all(&path).unwrap();

        // Call the function and check that it returns Ok
        assert!(run(path.to_str().unwrap()).is_ok());

        // Make sure the project has been created
        let buddy_file = path.join("Buddy.toml");
        assert!(buddy_file.exists());

        // Read the contents of the file
        let mut file_contents = String::new();
        fs::File::open(buddy_file)
            .expect("failed to open file")
            .read_to_string(&mut file_contents)
            .expect("failed to read file");

        // Assert that the file contents are equal to "geronimo"
        assert_eq!(
            file_contents,
            r#"[package]
name = "test_project"
version = "0.1.0"
edition = "2023"

[dependencies]
bazel-toolchain = "0.8.0"
google-test = "1.13.0""#
        );

        assert!(path.join("WORKSPACE").is_file());
        assert!(path.join("src").is_dir());
        assert!(path.join("test").is_dir());
    }

    #[test]
    fn test_run_on_non_existing_project() {
        let tmp_dir = tempfile::tempdir().unwrap();

        let path = tmp_dir.path().join("non-existing");

        // Call the function and check that it returns Ok
        assert!(run(path.to_str().unwrap()).is_ok());

        // Make sure the project has been created
        assert!(fs::metadata(path.join("Buddy.toml").to_str().unwrap()).is_ok());
    }

    #[test]
    fn test_run_on_existing_bazel_project() {
        let tmp_dir = tempfile::tempdir().unwrap();

        let path = tmp_dir.path().join("bazel-project");

        // Call the function and check that it returns Ok
        assert!(run(path.to_str().unwrap()).is_ok());

        // Make sure the project has been created
        assert!(fs::metadata(path.join("Buddy.toml").to_str().unwrap()).is_ok());
    }
}
