use clap::{Parser, Subcommand};
use colored::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufRead};
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use which::which;

pub mod commands;

fn new_package(package_name: &str, plugins: &[Plugin]) -> std::io::Result<()> {
    if !Path::new(package_name).exists() {
        println!(
            "    {} binary (application) `{}` package",
            "Created".green(),
            package_name
        );
        fs::create_dir(package_name)?;
        fs::create_dir(PathBuf::from(package_name).join("src"))?;
        fs::create_dir(PathBuf::from(package_name).join("test"))?;

        let mut file = File::create(PathBuf::from(package_name).join("WORKSPACE"))?;

        write!(
            file,
            r#"# This file is automatically @generated by Buddy.
# It is not intended for manual editing.
load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

"#
        )?;

        let build_rule = &plugins[0].build_rule;
        let build_rule = build_rule.replace("{version}", &plugins[0].versions["1.13.0"]);

        write!(file, "{}", build_rule)?;

        write!(file, "\n")?;

        let build_rule = &plugins[1].build_rule;

        write!(file, "{}", build_rule)?;

        let mut file = File::create(PathBuf::from(package_name).join("Buddy.toml"))?;
        write!(
            file,
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2023"

[dependencies]
bazel-toolchain = "0.8.0"
google-test = "1.13.0""#,
            package_name
        )?;

        let mut file = File::create(PathBuf::from(package_name).join("Buddy.lock"))?;
        write!(
            file,
            r#"# This file is automatically @generated by Buddy.
# It is not intended for manual editing.
version = 1

[[package]]
name = "google-test"
version = "1.13.0"
source = "https://github.com/google/googletest"
"#
        )?;

        let mut file = File::create(PathBuf::from(package_name).join(".bazelrc"))?;
        write!(file, r#"build --cxxopt=-std=c++17"#)?;
        write!(file, "\n")?;
        write!(
            file,
            r#"build --incompatible_enable_cc_toolchain_resolution"#
        )?;

        let mut file = File::create(PathBuf::from(package_name).join("src").join("BUILD"))?;

        write!(
            file,
            r#"load("@rules_cc//cc:defs.bzl", "cc_binary")

cc_binary(
    name = "{}",
    srcs = ["main.cc"],
)"#,
            package_name
        )?;

        let mut file = File::create(PathBuf::from(package_name).join("src").join("main.cc"))?;

        write!(
            file,
            r#"#include <ctime>
#include <string>
#include <iostream>

std::string get_greet(const std::string& who) {{
  return "Hello " + who;
}}

void print_localtime() {{
  std::time_t result = std::time(nullptr);
  std::cout << std::asctime(std::localtime(&result));
}}

int main(int argc, char** argv) {{
  std::string who = "world";
  if (argc > 1) {{
    who = argv[1];
  }}
  std::cout << get_greet(who) << std::endl;
  print_localtime();
  return 0;
}}"#
        )?;

        let mut file = File::create(PathBuf::from(package_name).join("test").join("BUILD"))?;

        write!(
            file,
            r#"cc_test(
  name = "hello_test",
  size = "small",
  srcs = ["hello_test.cc"],
  deps = ["@com_google_googletest//:gtest_main"],
)"#
        )?;

        let mut file = File::create(
            PathBuf::from(package_name)
                .join("test")
                .join("hello_test.cc"),
        )?;

        write!(
            file,
            r#"#include <gtest/gtest.h>

// Demonstrate some basic assertions.
TEST(HelloTest, BasicAssertions) {{
  // Expect two strings not to be equal.
  EXPECT_STRNE("hello", "world");
  // Expect equality.
  EXPECT_EQ(7 * 6, 42);
}}"#
        )?;

        Ok(())
    } else {
        println!(
            "{}: destination `{}` already exixts",
            "error".red(),
            package_name
        );
        Ok(())
    }
}

fn build(bazel_bin: &PathBuf, args: &[String]) -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::new(bazel_bin);

    // cmd.arg("--output_base=target/build");
    cmd.arg("build");
    cmd.arg("--symlink_prefix=target/");

    if args.len() != 0 {
        for arg in args {
            cmd.arg(arg);
        }
    } else {
        cmd.arg("//src/...");
    }

    let mut child = cmd
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    let stderr = child.stderr.take().unwrap();
    let reader = io::BufReader::new(stderr);

    for line in reader.lines() {
        let line = line.unwrap();
        if line.starts_with("INFO:") {
            let (_, message) = line.split_at(6);
            println!("{} {}", "INFO:".green(), message);
        } else {
            println!("{}", line);
        }
    }

    // Not sure why is still being generated. Eitherway, we get rid of it.
    let folder_path = Path::new("bazel-out");
    if folder_path.exists() {
        fs::remove_dir_all(folder_path).expect("Failed to delete folder");
    }

    Ok(())
}

fn run(bazel_bin: &PathBuf, args: &[String], config: &Config) -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::new(bazel_bin);

    // cmd.arg("--output_base=target/build");
    cmd.arg("run");
    cmd.arg("--symlink_prefix=target/");

    if args.len() != 0 {
        for arg in args {
            cmd.arg(arg);
        }
    } else {
        cmd.arg(format!("//src:{}", config.package.name));
    }

    let mut child = cmd
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    let stderr = child.stderr.take().unwrap();
    let reader = io::BufReader::new(stderr);

    for line in reader.lines() {
        let line = line.unwrap();
        if line.starts_with("INFO:") {
            let (_, message) = line.split_at(6);
            println!("{} {}", "INFO:".green(), message);
        } else {
            println!("{}", line);
        }
    }

    // Not sure why is still being generated. Eitherway, we get rid of it.
    let folder_path = Path::new("bazel-out");
    if folder_path.exists() {
        fs::remove_dir_all(folder_path).expect("Failed to delete folder");
    }

    Ok(())
}

fn test(bazel_bin: &PathBuf, args: &[String]) -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::new(bazel_bin);

    // cmd.arg("--output_base=target/build");
    cmd.arg("test");
    cmd.arg("--test_output=all");
    cmd.arg("--symlink_prefix=target/");

    if args.len() != 0 {
        for arg in args {
            cmd.arg(arg);
        }
    } else {
        cmd.arg("//test/...");
    }

    let mut child = cmd
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    let stderr = child.stderr.take().unwrap();
    let reader = io::BufReader::new(stderr);

    for line in reader.lines() {
        let line = line.unwrap();
        if line.starts_with("INFO:") {
            let (_, message) = line.split_at(6);
            println!("{} {}", "INFO:".green(), message);
        } else {
            println!("{}", line);
        }
    }

    // Not sure why is still being generated. Eitherway, we get rid of it.
    let folder_path = Path::new("bazel-out");
    if folder_path.exists() {
        fs::remove_dir_all(folder_path).expect("Failed to delete folder");
    }

    Ok(())
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new buddy package
    New { path: String },

    /// Create a new buddy package in an existing directory
    Init {
        #[clap(default_value = ".")]
        path: String,
    },

    /// Compile the current package
    Build { targets: Vec<String> },

    /// Run a binary or example of the local package
    Run { targets: Vec<String> },

    /// Run the tests
    Test { targets: Vec<String> },
}

#[derive(Debug, Deserialize, Default)]
struct Package {
    name: String,
    version: String,
    edition: String,
}

#[derive(Debug, Deserialize, Default)]
struct Config {
    package: Package,
    dependencies: HashMap<String, String>,
}

#[derive(Debug)]
struct Plugin {
    name: String,
    versions: HashMap<String, String>,
    build_rule: String,
}

fn main() {
    let cli = Cli::parse();

    let bazel_bin = match which("bazelisk") {
        Ok(path) => path,
        Err(_) => panic!("Bazelisk binary not found. See https://docs.bazel.build/versions/5.4.1/install-bazelisk.html"),
    };

    let file_path = "Buddy.toml";
    let config: Config = match fs::read_to_string(file_path) {
        Ok(content) => toml::from_str(&content).unwrap(),
        Err(_) => Config::default(),
    };

    println!("{:#?}", config);

    let plugins = vec![
        Plugin {
            name: "google-test".to_string(),
            versions: [
                (
                    "1.13.0".to_string(),
                    "b796f7d44681514f58a683a3a71ff17c94edb0c1".to_string(),
                ),
                (
                    "1.12.1".to_string(),
                    "58d77fa8070e8cec2dc1ed015d66b454c8d78850".to_string(),
                ),
            ]
            .iter()
            .cloned()
            .collect(),
            build_rule:  r#"http_archive(
  name = "com_google_googletest",
  urls = ["https://github.com/google/googletest/archive/5ab508a01f9eb089207ee87fd547d290da39d015.zip"],
  strip_prefix = "googletest-5ab508a01f9eb089207ee87fd547d290da39d015",
)"#.to_string(),
        },
        Plugin {
            name: "bazel-toolchain".to_string(),
            versions: [
                (
                    "0.8.2".to_string(),
                    "b796f7d44681514f58a683a3a71ff17c94edb0c1".to_string(),
                ),
                (
                    "1.12.1".to_string(),
                    "58d77fa8070e8cec2dc1ed015d66b454c8d78850".to_string(),
                ),
            ]
            .iter()
            .cloned()
            .collect(),
            build_rule:  r#"BAZEL_TOOLCHAIN_TAG = "0.8.2"
BAZEL_TOOLCHAIN_SHA = "0fc3a2b0c9c929920f4bed8f2b446a8274cad41f5ee823fd3faa0d7641f20db0"

http_archive(
    name = "com_grail_bazel_toolchain",
    sha256 = BAZEL_TOOLCHAIN_SHA,
    strip_prefix = "bazel-toolchain-{tag}".format(tag = BAZEL_TOOLCHAIN_TAG),
    canonical_id = BAZEL_TOOLCHAIN_TAG,
    url = "https://github.com/grailbio/bazel-toolchain/archive/refs/tags/{tag}.tar.gz".format(tag = BAZEL_TOOLCHAIN_TAG),
)

load("@com_grail_bazel_toolchain//toolchain:deps.bzl", "bazel_toolchain_dependencies")

bazel_toolchain_dependencies()

load("@com_grail_bazel_toolchain//toolchain:rules.bzl", "llvm_toolchain")

llvm_toolchain(
    name = "llvm_toolchain",
    llvm_version = "15.0.6",
)

load("@llvm_toolchain//:toolchains.bzl", "llvm_register_toolchains")

llvm_register_toolchains()"#.to_string(),
        }
    ];

    match &cli.command {
        Commands::New { path } => new_package(&path, &plugins).unwrap(),
        Commands::Init { path } => commands::init::run(&path)
            .unwrap_or_else(|error| println!("{}: {}", "error".red(), error)),
        Commands::Build { targets } => build(&bazel_bin, &targets).unwrap(),
        Commands::Run { targets } => run(&bazel_bin, &targets, &config).unwrap(),
        Commands::Test { targets } => test(&bazel_bin, &targets).unwrap(),
    }

    println!("{:#?}", plugins);
}