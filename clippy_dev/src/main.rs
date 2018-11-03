// Copyright 2014-2018 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


extern crate clap;
extern crate clippy_dev;
extern crate regex;

use clap::{App, Arg, SubCommand};
use clippy_dev::*;

fn main() {
    let matches = App::new("Clippy developer tooling")
        .subcommand(
            SubCommand::with_name("update_lints")
                .about("Makes sure that:\n \
                       * the lint count in README.md is correct\n \
                       * the changelog contains markdown link references at the bottom\n \
                       * all lint groups include the correct lints\n \
                       * lint modules in `clippy_lints/*` are visible in `src/lib.rs` via `pub mod`\n \
                       * all lints are registered in the lint store")
                .arg(
                    Arg::with_name("print-only")
                        .long("print-only")
                        .short("p")
                        .help("Print a table of lints to STDOUT. This does not include deprecated and internal lints. (Does not modify any files)"),
                )
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("update_lints") {
        if matches.is_present("print-only") {
            print_lints();
        } else {
            update_lints();
        }
    }
}

fn print_lints() {
    let lint_list = gather_all();
    let usable_lints: Vec<Lint> = Lint::usable_lints(lint_list).collect();
    let lint_count = usable_lints.len();
    let grouped_by_lint_group = Lint::by_lint_group(&usable_lints);

    for (lint_group, mut lints) in grouped_by_lint_group {
        if lint_group == "Deprecated" { continue; }
        println!("\n## {}", lint_group);

        lints.sort_by_key(|l| l.name.clone());

        for lint in lints {
            println!("* [{}]({}#{}) ({})", lint.name, clippy_dev::DOCS_LINK.clone(), lint.name, lint.desc);
        }
    }

    println!("there are {} lints", lint_count);
}

fn update_lints() {
    let lint_list: Vec<Lint> = gather_all().collect();
    let usable_lints: Vec<Lint> = Lint::usable_lints(lint_list.clone().into_iter()).collect();
    let lint_count = usable_lints.len();

    replace_region_in_file(
        "../README.md",
        r#"\[There are \d+ lints included in this crate!\]\(https://rust-lang-nursery.github.io/rust-clippy/master/index.html\)"#,
        "",
        true,
        || {
            vec![
                format!("[There are {} lints included in this crate!](https://rust-lang-nursery.github.io/rust-clippy/master/index.html)", lint_count)
            ]
        }
    );

    replace_region_in_file(
        "../CHANGELOG.md",
        "<!-- begin autogenerated links to lint list -->",
        "<!-- end autogenerated links to lint list -->",
        false,
        || { gen_changelog_lint_list(lint_list.clone()) }
    );

    replace_region_in_file(
        "../clippy_lints/src/lib.rs",
        "begin deprecated lints",
        "end deprecated lints",
        false,
        || { gen_deprecated(&lint_list) }
    );

    replace_region_in_file(
        "../clippy_lints/src/lib.rs",
        "begin lints modules",
        "end lints modules",
        false,
        || { gen_modules_list(lint_list.clone()) }
    );

    // Generate lists of lints in the clippy::all lint group
    replace_region_in_file(
        "../clippy_lints/src/lib.rs",
        r#"reg.register_lint_group\("clippy::all""#,
        r#"\]\);"#,
        false,
        || {
            // clippy::all should only include the following lint groups:
            let all_group_lints = usable_lints.clone().into_iter().filter(|l| {
                l.group == "correctness" ||
                  l.group == "style" ||
                  l.group == "complexity" ||
                  l.group == "perf"
            }).collect();

            gen_lint_group_list(all_group_lints)
        }
    );

    // Generate the list of lints for all other lint groups
    for (lint_group, lints) in Lint::by_lint_group(&usable_lints) {
        replace_region_in_file(
            "../clippy_lints/src/lib.rs",
            &format!("reg.register_lint_group\\(\"clippy::{}\"", lint_group),
            r#"\]\);"#,
            false,
            || { gen_lint_group_list(lints.clone()) }
        );
    }
}