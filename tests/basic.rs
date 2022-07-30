

// #[macro_use]
use reframe::core::{*};



fn build_config(name: &str) -> Config {
    Config {
        reframe: ReframeConfig {
            name: "My Reframe".to_string(),
            author: "robin".to_string(),
            min_version: "0.1.0".to_string(),
        },
        project: ProjectConfig {
            name: name.to_owned(),
            variants: Default::default(),
            version: "0.1.1".to_string(),
            ignore_dirs: None,
            ignore_files: None,
            finish_text: None,
        },
        param: vec![],
        presents: vec![],
        post_generate: vec![],
    }
}

#[test]
fn test_if_conditional() {
    let input = r#"\
        project = "$name$";
        # <% if param.with_x %>
        import x;
        # <% endif %>
        # <% if param.db == "sqlite" %>
        import sqlite;
        # <% endif %>
        # <% if param.db == "mysql" %>
        import mysql;
        # <% endif %>
        "#;

    let expected1 = r#"\
        project = "Conditional";
        import sqlite;
        "#;
    let expected2 = r#"\
        project = "Conditional";
        import x;
        import sqlite;
        "#;
    let expected3 = r#"\
        project = "Conditional";
        import mysql;
        "#;

    let name = "Conditional";

    let config = build_config(name);

    let p = Param::new("db".to_string(), "sqlite".to_owned());

    let mut param = vec![];
    param.push(p);

    let output = Reframe::process_template_str(input.to_string(), &config, &param, &[]);
    assert_eq!(output, expected1);

    param.push(Param::new("with_x".to_string(), "true".to_owned()));
    let output = Reframe::process_template_str(input.to_string(), &config, &param, &[]);
    assert_eq!(output, expected2);

    param.clear();
    param.push(Param::new("with_x".to_string(), "false".to_owned()));
    param.push(Param::new("db".to_string(), "mysql".to_owned()));
    let output = Reframe::process_template_str(input.to_string(), &config, &param, &[]);
    assert_eq!(output, expected3);
}

#[test]
fn test_if_conditional_comment_mark() {
    let input = r#"\
        project = "$name$";
        -- <% if param.with_x %>
        import x;
        -- <% endif %>
        // <% if param.db == "sqlite" %>
        import sqlite;
        // <% endif %>
        # <% if param.db == "mysql" %>
        import mysql;
        # <% endif %>
        "#;

    let expected1 = r#"\
        project = "Conditional";
        import sqlite;
        "#;

    let name = "Conditional";

    let config = build_config(name);

    let p = Param::new("db".to_string(), "sqlite".to_owned());

    let mut param = vec![];
    param.push(p);
    param.push(Param::new("with_x".to_string(), "false".to_owned()));

    let output = Reframe::process_template_str(input.to_string(), &config, &param, &[]);
    assert_eq!(output, expected1);
}

#[test]
fn test_if_conditional_sql() {
    let input = r#"\
        -- mulai akun
        -- <% if param.with_account %>
        CREATE TABLE accounts (
            id BIGSERIAL PRIMARY KEY,
            full_name VARCHAR NOT NULL,
            email VARCHAR NOT NULL
        );
        -- <% endif %>
        -- selesai
        "#;

    let expected1 = r#"\
        -- mulai akun
        -- selesai
        "#;

    let name = "Conditional";

    let config = build_config(name);

    let mut param = vec![];
    param.push(Param::new("with_account".to_string(), "false".to_owned()));

    let output = Reframe::process_template_str(input.to_string(), &config, &param, &[]);
    assert_eq!(output, expected1);
}

#[test]
#[should_panic(expected = "unclosed if conditional `# <% if param.with_x %>` at line 2")]
fn test_unclosed_if_tag() {
    let input = r#"\
        project = "$name$";
        # <% if param.with_x %>
        import x;
        import sqlite;
        "#;

    let config = build_config("any");

    let param = vec![];
    let _ = Reframe::process_template_str(input.to_string(), &config, &param, &[]);
}
