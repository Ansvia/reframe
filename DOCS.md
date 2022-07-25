Documentation
==========

Create file `Reframe.toml` at the root project dir, example:

```toml
[reframe]
# Name of this template
name = "Hello World"
author = "anvie"

# minimum Reframe version to be used
min_version = "0.4.0"

[project]
# Default name of the project.
name = "HelloWorld"

# Default project version
version = "0.1.0"

# Don't process these directives
ignore_dirs = [
    "target", "build"
]

# These following text usage will be shown when generation finised and succeed.
finish_text = """Usage:
    $ cd $name_kebab_case$
Install prerequisites:
    $ pip install -r requirements.txt
Test:
    $ python ./scripts/test.py
Deploy:
    $ python ./scripts/deploy.py
"""

# all parameters bellow will be asked from user before the generation takes place.
# You can add multiple parameters.
# the following parameters will be available in template:

# example usage in source: $param.description$
[[param]]
description = { ask = "Description ?", default = "My simple project" }

[[param]]
author_name = { ask = "Author name?", default="Author" }

[[param]]
author_email = { ask = "Author email?", default="author@example.com" }

[[param]]
with_web_frontends = { ask = "With web frontends?", default = "false" }

# conditional param if `with_web_frontends` is true.
[[param]]
with_typescript = { ask = "Use typescript?", default = "false", if="with_web_frontends" }

# `present` keyword ensures that directory or file is present according to the condition
# from the parameter above, if `with_web_frontends` param is false then the `frontends/web`
# directory will be removed, otherwise it will be kept in place and subject to processing.
[[present]]
path = "frontends/web"
if = "with_web_frontends"

# `post_generate` keyword allows you to run a command after the generation is finished.
# currently only supports `make_executable` command which make the specific file executable.
[[post_generate]]
make_executable="./scripts/run.sh"
```
