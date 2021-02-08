Reframe
==========

![Crates](https://img.shields.io/crates/v/reframe.svg) [![Build Status](https://travis-ci.org/Ansvia/reframe.svg?branch=master)](https://travis-ci.org/Ansvia/reframe)

> Because *"don't repeat yourself"*

If you need to build a project in matter of seconds, this tool for you.

![Reframe Demo](img/reframe.gif?raw=true)

Install
----------

Download binary file for your specific platform from [release page](https://github.com/Ansvia/reframe/releases).

Or, if you are Rust and Cargo user, type:

    $ cargo install reframe

Usage
--------

    $ reframe [SOURCE]

Example
---------

    $ reframe anvie/basic-rust

`anvie/basic-rust` is refering to my github repo: [basic-rust.rf](https://github.com/anvie/basic-rust.rf).

Build Template
----------------

To create Reframe template is super duper easy, all you needs is write file called `Reframe.toml` and place in root project dir, example:

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

After finish writing project template you can test using `reframe [YOUR-WORKING-TEMPLATE-DIR]`,
if all is ok, push to your github repo with additional postfix `.rf` at it repo name, eg: if your repo name is `unicorn` then you must push with name `unicorn.rf`, and finally you can use by simply typing:

    $ reframe [MY-GITHUB-USERNAME]/[MY-TEMPLATE]

Example:

    $ reframe agus/unicorn

For more detail please see working examples:
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


"Ready for Work" sources
-----------------------

* [ansvia/mainframe.rf](https://github.com/ansvia/mainframe.rf) - The most advanced full-stack framework.
* [anvie/basic_cli.rf](https://github.com/anvie/basic_cli.rf) - Basic CLI application.
* [anvie/rust-grpc.rf](https://github.com/anvie/rust-grpc.rf) - Rust gRPC application.
* [anvie/substrate-erc20.rf](https://github.com/anvie/substrate-erc20.rf) - ERC-20 Smart Contract.


