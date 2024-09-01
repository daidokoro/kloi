[![Build kloi](https://github.com/daidokoro/kloi/actions/workflows/release.yaml/badge.svg)](https://github.com/daidokoro/kloi/actions/workflows/release.yaml)

# kloi

 <p align="center">
  <img src="misc/kloi-logo.png" width="500">
</p>

kloi is the successor to the [qaz](https://github.com/daidokoro/qaz) project. It was created to be a simple Cloudformation template manager. It deploys, checks, deletes and updates Cloudformation stacks based on a configuration file.

Like [qaz](https://github.com/daidokoro/qaz), kloi supports templating, however, kloi uses the the [Handlerbars]() templating framework, more on that [here](#templating)

### Why?

Using AWS CLI to deploy, delete or update Cloudformation stacks when rapidly testing builds is annonying, especially when dealing with complex templates with multiple stacks and parameters.

kloi allows you to add cloudformation parameters, template values and other configuration needed to a single dynamic configuration file and use this file as a manifest for how your stacks should be defined and handled.

<p align="center">
  <img src="misc/kloi-example.gif">
</p>

---

<details>
<summary><b>Table of Contents</b></summary>

- **[Installation](#installation)**
  - [MacOs](#macos)
  - [Linux](#linux)
  - [Rust](#)
- **[Configuration](#configuration)**
  - [Modules](#modules)
    - [stacks](#stacks)
      - [new](#new)
      - [add](#add)
    - [os](#os)
      - [open](#open)
      - [cmd](#cmd)
      - [env](#env)
    - [http](#http)
      - [get](#get)
      - [post](#post)
  - [Templating](#templating)
- **[Usage](#usage)**

</details>

---

### Installation

#### MacOs & Linux

```sh
curl -s https://raw.githubusercontent.com/daidokoro/kloi/main/install.sh | bash
```

or with `sudo` if required.

```sh
curl -s https://raw.githubusercontent.com/daidokoro/kloi/main/install.sh | sudo bash
```

#### Cargo

```sh
cargo install --git https://github.com/daidokoro/kloi kloi
```

### Configuration

Kloi configuration files are written in [Starlark](https://github.com/bazelbuild/starlark), a python-like dialect developed to be used as a configuration language.

Configuration files are specified using the `-c/--config` flag or using the `KLOI_CONFIG` env variable.

Configuration files can be read remotely via HTTP or locally from the file system.

```sh
# using KLOI_CONFIG env variable
export KLOI_CONFIG='https://path/to/config.star '
kloi apply <stack-name>

# or --config flag
kloi apply <stack-name> --config https://path.to/config.star

# or local file
kloi apply <stack-name> --config path/to/config.star
```

Note the KLOI_CONFIG env variable takes precedence over the `--config` flag.

By allowing remote configuration files, kloi can be used to centralise cloudformation templates and configurations across multiple projects.

There are a few internal configuration functions/modules to be aware of when writing a kloi configuration file.

#### Modules

Kloi configuration contains a few built-in modules. These modules contain support functions that add functionality to the configuration process.

Functions in kloi config are called using the following syntax:

```
<module>.<function_name>
```

##### Stacks

This module allows you to define and add cloudformation stacks to your workflow. It has the following functions

###### new

Creates a new stack object

| args             | required | type           | desc                                                                                                                                                                                                                                        |
|------------------|----------|----------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| name             | ✓        | `string`       | The name given to your stack, this name will be used to set the stack name when deploying and is also the name used when referring to the stack on the cli, for eg:<br>`kloi  apply <name>`                                                                                                                                                   |
| region           | ✓        | `string`       | The region to deploy your stack. Note that regions are not global, a region must be specified per stack. In this way, `kloi` is able to make manages stacks across multiple regions.                                                                                                                                     |
| template         | ✓        | `string`       | The cloudformation template string                                                                                                                                                                                                          |
| parameters       |          | `dict`         | A dictionary *(key/value pair)* containing the Parameter Names and Values to pass to the cloudformation template                                                                                                                            |
| capabilities     |          | `list<string>` | Depending on the resources being deployed by your cloudformation template, specific IAM capabilities may be required. <br><br>Allowed Values:<br> - CAPABILITY_IAM<br> - CAPABILITY_NAMED_IAM<br> - CAPABILITY_AUTO_EXPAND                  |
| bucket           |          | `string`       | If your template exceeds the direct deployment limit, you must specify a bucket to upload your s3 template for deployment.                                                                                                                  |
| values           |          | `dict`         | This argument allows you to specify a **dict** *(key/value pair)* containing the values that will be expanded using templating. See more on templating [here]().                                                                            |
| custom_resources |          | `list<string>` | A list of Cloudformation Custom Resources that are created by this deployment. If specified, the logs from these Lambda Custom Resources will be collected and printed to stdout each time the stack is **created, updated or deleted**<br> |

> returns: type (stack)

*usage:*

```python
# read template file from file system.
template = os.open('path/to/cfn/template.yaml')

# define a stack
my_stack = stacks.new(
    name = 'stack',
    region = 'eu-west-1',
    template = template,
    paramaeters = {
      'VPCID': '123456789',
      'PolicyName': 'MyPolicy'
    },
    capabilities = [
      'CAPABILITY_IAM'
    ],
    bucket = 'bucket'
)
```

##### add

Adds a stack to the kloi configuration

| args  | required | type                                                    |
|-------|----------|---------------------------------------------------------|
| stack | ✓        | `type(stack)` <br>*type returned by* [stacks.new](#new) |

*usage:*

```python
# create stack
my_stack = stacks.new(
    name = "stack-name",
    template = "path/to/template.yml",
    parameters = {
        "param1": "value1",
        "param2": "value2"
    }
)

# add stack
stacks.add(my_stack)
```

Note that stacks must be added in order for them to be managed. Stacks that are defined but not added, will be ignored.

---

##### os

This module contains functions related to the Operating System. If any os function fails to execute successfully, kloi will **panic**.

###### open

The open function reads a file at the given path and returns its contents as a *string*.

| args | required | type     |
|------|----------|----------|
| path | ✓        | `string` |

> returns: string

*usage:*

```python
# read template file from file system.
template = os.open('path/to/cfn/template.yaml')

my_stack = stacks.new(
  name = 'my_stack',
  template = template,
  region = 'eu-west-1'
)
```

###### cmd

The cmd function allows you to execute commands on the host. This can be used to run scripts or execute specific commands required to build your template.

| args    | required | type     |
|---------|----------|----------|
| command | ✓        | `string` |

> returns: string

*usage:*

```python
# run script to generate cloudformation template
template = os.cmd('my/template/generator.sh')
template_values = os.cmd('my/values.sh')

my_stack = stacks.new(
  name = 'my_stack',
  region = 'eu-west-1',
  template = template,
  values = values
)
```

###### env

The env function reads a given ENV Variable and returns its value if set.

| args | required | type     |
|------|----------|----------|
| name | ✓        | `string` |

> returns: string

*usage:*

```python
# use environment variable in config
my_region = os.env('REGION')

# use the value in a stack definition
my_stack = stacks.new(
  name = "my_stack",
  region = my_region,
  template = ''
)

## --- OR

my_stack = stacks.new(
  name = "my_stack",
  region = os.env('REGION'),
  template = ''
)
```

---

##### http

The http module contains functions for calling HTTP endpoints

###### get

The get function is used to execute HTTP GET request for a given URL

| args    | required | type     | desc                                                                            |
|---------|----------|----------|---------------------------------------------------------------------------------|
| headers |          | `dict`   | A dictionary *(key/value pair)* containing the headers to send with the request |
| url     | ✓        | `string` | The URL to call                                                                 |

> returns: string

*usage:*

```python
# call with headers to get template
template = http.get(
    headers={"some": "value"}, 
    url='http://my.template.source')

my_stack = stacks.new(
  name = 'my_stack',
  region = 'eu-west-1',
  template = template,
)

## --- OR
my_stack = stacks.new(
  name = 'my_stack',
  region = 'eu-west-1',
  # call without headers
  template = http.get('http://my.template.source'),
)
```

###### post

The post function is used to execute HTTP POST requests for a given URL and payload

| args    | required | type     | desc                                                                            |
|---------|----------|----------|---------------------------------------------------------------------------------|
| headers |          | `dict`   | A dictionary *(key/value pair)* containing the headers to send with the request |
| url     | ✓        | `string` | The URL to call                                                                 |
| body    | ✓        | `dict`   | The payload to send with the request                                            |

> returns: string

*usage:*

```python
# call with headers
template = http.get(
  headers={"some": "value"}, 
  url='http://my.template.source', 
  body: {"some": "payload"})

my_stack = stacks.new(
  name = 'my_stack',
  region = 'eu-west-1',
  template = template,
)

## --- OR
my_stack = stacks.new(
  name = 'my_stack',
  region = 'eu-west-1',
  # call without headers
  template = http.post(
    'http://my.template.source',
    {"some": "payload"}
  )
)
```

---

### Templating

Kloi uses the [Handlerbars](https://handlebarsjs.com/guide/#what-is-handlebars) templating framework to expand values in the configuration file. This allows you to use variables in your configuration file that can be expanded at runtime.

As mentioned above, the `values` argument in the `stacks.new` function is used to specify a dictionary containing the values that will be expanded using templating.

*example:*

> config.star:

```python
region = 'eu-central-1'
values = {
    "cidrs": ["10.10.10.0/24", "10.10.10.1/24"],
    "subnet": {
        "region": region
    }
}

stack = stacks.new(
    name = 'ops',
    region = region,
    template = os.open('./template.yaml'),
    values = values,
    capabilities = [
        'CAPABILITY_AUTO_EXPAND',
        'CAPABILITY_IAM'
    ]
)

# add the stack to the list of stacks
stacks.add(stack)
```

> template.yaml:

```hbs
AWSTemplateFormatVersion: '2010-09-09'
Transform: AWS::Serverless-2016-10-31

Resources:
{{#each cidrs}}
  VPC{{ @index }}:
    Type: AWS::EC2::VPC
    Properties:
      CidrBlock: {{ this }}
      EnableDnsSupport: 'true'
      EnableDnsHostnames: 'true'
      Tags:
      - Key: stack
        Value: kloi

{{/each}}

{{#if subnet }}
{{#each cidrs}}
  MySubnet:
    Type: 'AWS::EC2::Subnet'
    Properties:
      # AvailabilityZone: {{ ../subnet.region }}
      VpcId: !Ref VPC{{@index}}
      CidrBlock: {{ this }}

{{/each}}
{{/if}}
```

Next we generate the template using the `show` command

```sh
$ kloi show -c config.star ops
```

```yaml
AWSTemplateFormatVersion: '2010-09-09'
Transform: AWS::Serverless-2016-10-31

Resources:
  VPC0:
    Type: AWS::EC2::VPC
    Properties:
      CidrBlock: 10.10.10.0/24
      EnableDnsSupport: 'true'
      EnableDnsHostnames: 'true'
      Tags:
      - Key: stack
        Value: kloi

  VPC1:
    Type: AWS::EC2::VPC
    Properties:
      CidrBlock: 10.10.10.1/24
      EnableDnsSupport: 'true'
      EnableDnsHostnames: 'true'
      Tags:
      - Key: stack
        Value: kloi


  MySubnet:
    Type: 'AWS::EC2::Subnet'
    Properties:
      # AvailabilityZone: eu-central-1
      VpcId: !Ref VPC0
      CidrBlock: 10.10.10.0/24

  MySubnet:
    Type: 'AWS::EC2::Subnet'
    Properties:
      # AvailabilityZone: eu-central-1
      VpcId: !Ref VPC1
      CidrBlock: 10.10.10.1/24
```

Note that the `{{#each}}` and `{{#if}}` blocks are used to iterate over the `cidrs` array and check if the `subnet` object is defined in the `values` dictionary. The result shows that the vpc and subnet resources are generated based on the values in the `cidrs` array and the `subnet` object.

---

### Usage

#### apply / delete

Kloi has a few commands that can be used to manage your cloudformation stacks. 

- **apply**: Deploys/Updates a cloudformation stack

```sh
$ kloi apply <stack-name> --config <path/to/config>


```

- **delete**: Deletes a stack from AWS Cloudformation

```sh
$ kloi delete <stack-name> --config <path/to/config>
```

You can use the environment variable `KLOI_CONFIG` to set the path to your configuration file. This will allow you to run kloi commands without specifying the `--config` flag.

```sh
$ kloi apply <stack-name>
$ kloi delete <stack-name>
```

If you're not sure what the stack names are in your configuration file, you can run the `kloi apply` or `kloi delete` commands without any arguments to get an interactive list of stacks to choose from.

<p align="center">
  <img src="misc/kloi-interactive-example.gif">
</p>


#### show

To view the cloudformation template associated with the stack, you can use the `show` command.

```sh
$ kloi show <stack-name> --config <path/to/config>
```

This is useful for debugging and verifying that the template is correct before deploying, especially when using templating values to expand the template dynamically.


<p align="center">
  <img src="misc/kloi-show-command.gif">
</p>


#### debug

Debug logs can be enabled by setting the `KLOI_LOG` environment variable to `debug`.

```sh
$ export KLOI_LOG=debug
$ kloi apply <stack-name>

# or
$ KLOI_LOG=debug kloi apply <stack-name>
```

---

Kloi is in **beta** and is still under active development. If you find any issues or have any feature requests, please open an issue.

TODO:

- [ ] CI/CD
  - [X] Automate Release workflow
  - [ ] Integration Tests on PR
  - [ ] Unit Tests on PR
- [ ] Documentation
  - [ ] Add more examples
  - [ ] Add more detailed usage
- [ ] Features
  - [ ] Value and Parameter override from the CLI
  - [ ] Improve `kloi status` command output, show drifts and other stack information
  - [ ] `kloi copy` command to copy existing cloudformation stacks into a kloi config.
