use heck::*;

use crate::core::*;

use std::collections::HashMap;

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
    let input = r#"
{{#if with_capped}}
bool capped = true;
{{/if}}
"#;

    let expected1 = r#"
"#;
    let expected2 = r#"
bool capped = true;
"#;

    let name = "Conditional";

    let config = build_config(name);

    // let p = Param::new("db".to_string(), "sqlite".to_owned());

    let mut param = vec![];
    // param.push(p);

    let output = Reframe::process_with_handlebars(
        "ConditionalTest",
        input.to_string(),
        &config,
        &param,
        &[],
    )
    .unwrap();
    assert_eq!(output, expected1);

    param.push(Param::new("with_capped".to_string(), "true".to_owned()));
    let output = Reframe::process_with_handlebars(
        "ConditionalTest",
        input.to_string(),
        &config,
        &param,
        &[],
    )
    .unwrap();
    assert_eq!(output, expected2);

    param.clear();
    // param.push(Param::new("with_x".to_string(), "false".to_owned()));
    // let output = Reframe::process_template_str(input.to_string(), &config, &param, &[]);
    // assert_eq!(output, expected3);
}

#[test]
fn test_if_conditional_nested() {
    let input = r#"
{{#if with_capped}}
bool capped = true;
int maxTotal = 10;
{{/if}}
{{#if with_something}}
function doSomething(){
    {{#if with_capped}}
    require(count <= maxTotal);
    {{/if}}
    return true;
}
{{/if}}
"#;

    let expected1 = r#"
"#;
    let expected2 = r#"
bool capped = true;
int maxTotal = 10;
"#;
    let expected3 = r#"
bool capped = true;
int maxTotal = 10;
function doSomething(){
    require(count <= maxTotal);
    return true;
}
"#;
    let expected4 = r#"
function doSomething(){
    return true;
}
"#;

    let name = "Conditional";

    let config = build_config(name);

    let mut param = vec![];

    let output = Reframe::process_with_handlebars(
        "ConditionalTest",
        input.to_string(),
        &config,
        &param,
        &[],
    )
    .unwrap();
    assert_eq!(output, expected1);

    param.push(Param::new("with_capped".to_string(), "true".to_owned()));
    let output = Reframe::process_with_handlebars(
        "ConditionalTest",
        input.to_string(),
        &config,
        &param,
        &[],
    )
    .unwrap();
    assert_eq!(output, expected2);

    param.push(Param::new("with_something".to_string(), "true".to_owned()));
    let output = Reframe::process_with_handlebars(
        "ConditionalTest",
        input.to_string(),
        &config,
        &param,
        &[],
    )
    .unwrap();
    assert_eq!(output, expected3);

    param.clear();
    param.push(Param::new("with_something".to_string(), "true".to_owned()));
    let output = Reframe::process_with_handlebars(
        "ConditionalTest",
        input.to_string(),
        &config,
        &param,
        &[],
    )
    .unwrap();
    assert_eq!(output, expected4);
}
