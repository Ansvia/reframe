Reframe
==========

[![Crates](https://img.shields.io/crates/v/reframe.svg)](https://crates.io/crates/reframe) [![Build Status](https://travis-ci.org/Ansvia/reframe.svg?branch=master)](https://travis-ci.org/Ansvia/reframe)

> Because *"don't repeat yourself"*

Reframe is a lightweight project scaffolding tool enables rapid setup of new projects by generating the necessary directories, files, and code templates, streamlining the development process from the outset.

![Reframe Demo](img/reframe.gif?raw=true)

For detail usage please check [Reframe Documentation](DOCS.md).

Install
----------

### Homebrew

For Mac with homebrew:

```bash
brew tap ansvia/tools
brew install reframe
```

### Cargo

Or, if you have Cargo, type:

    $ cargo install reframe

### Download binary

Download binary for your specific platform from [release page](https://github.com/Ansvia/reframe/releases).


Usage
--------

    $ reframe [SOURCE]

Example
---------

    $ reframe anvie/basic-rust

`anvie/basic-rust` is refering to my github repo: [basic-rust.rf](https://github.com/anvie/basic-rust.rf).

Build Source
----------------

To create Reframe source is super duper easy, all you needs is write `Reframe.toml` at the root project dir, example:

```toml
[project]
name = "Hello World"
version = "1.0"

[[param]]
with_serde = { ask = "Dengan serde?", default = false }

[[param]]
serde_version = { ask = "Versi serde?", default = "1.0", if="with_serde" }

[[param]]
# without default value means required
author_name = { ask = "Author name?" }

[[param]]
author_email = { ask = "Author email?" }
```

Every string type param will have case variants automagically, eg: `author_name` will have: `author_name_lowercase`, `author_name_snake_case`, `author_name_kebab_case`.

So when you need to get project name with snake case, write: $name_snake_case$.

When you done, you can test using `reframe [YOUR-WORKING-TEMPLATE-DIR]`,
if all is ok, push the project to your github repo with additional postfix `.rf` at the project name, eg: if your repo name is `unicorn` then you must push with name `unicorn.rf`, and finally you can use anywhere by simply typing:

    $ reframe [MY-GITHUB-USERNAME]/[MY-TEMPLATE]

Example:

    $ reframe agus/unicorn

For detail usage please check [Reframe Documentation](DOCS.md).

Reframe source examples:
* [anvie/basic-rust.rf](https://github.com/anvie/basic-rust.rf).
* [anvie/hello-world-py.rf](https://github.com/anvie/hello-world-py.rf)

Supported case variants:

* `*_lower_case` -> my cool app
* `*_snake_case` -> my_cool_app
* `*_kebab_case` -> my-cool-app
* `*_shout_snake_case` -> MY_COOL_APP
* `*_upper_case` -> MY COOL APP
* `*_camel_case` -> myCoolApp
* `*_pascal_case` -> MyCoolApp

You can also use builtin variables:

* `year` -> Print current year, eg: 2019.
* `month_name` -> Print current month, eg: July


Templating
------------

Reframe also support templating engine for manipulating code use Handlebars syntax, example:

```javascript
{{#if with_jwt}}
const jwt = require('jsonwebtoken');
{{/if}}
```

Available sources:
-----------------------

* [anvie/basic-rust-cli.rf](https://github.com/anvie/basic-rust-cli.rf) - Basic CLI application.
* [anvie/rust-grpc.rf](https://github.com/anvie/rust-grpc.rf) - Rust gRPC application.

For more sources see [SOURCES](SOURCES.md).

You can also list available sources by typing:

```bash
reframe --list
```
