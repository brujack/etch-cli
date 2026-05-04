# Packages

- package.install
- package.repository

## Package Providers

Packages is a group of actions that utilize the local system's package manager. Unlike some other actions, packages can also contain _providers_. Some operating systems may have multiple package managers available and providers allow the user to choose which package manager to use.

### Supported package providers

| Provider         | OS            |
| :--------------- | :------------ |
| apt / apt-get    | Ubuntu/Debian |
| brew / homebrew  | macOS         |
| snap / snapcraft | Linux         |

If you would like support added for another package provider, open an issue at the [repository](https://github.com/brujack/etch-cli).

### Important note on homebrew and macOS

Some package manager providers implement a `bootstrap` method that will automatically configure the package manager if it is not part of the default installation. etch can automatically install `homebrew` to a macOS system if a manifest specifies a `package.install` action without overriding the macOS default.

## package.install

| Key        | Type   | Optional | Description                                                                 |
| :--------- | :----- | :------- | :-------------------------------------------------------------------------- |
| action     | string | no       | `package.install`                                                           |
| name       | string | no       | name of target package                                                      |
| list       | list   | yes      | list of multiple packages                                                   |
| provider   | string | yes      | Specify package provider                                                    |
| repository | string | yes      | specific repository for a provider and package                              |
| file       | bool   | yes      | Specify that package is a local package on the file system. Default `false` |

### Example

```yaml
# Install package using default provider
- action: package.install
  name: curl

# Install a list of packages using default provider
- action: package.install
  list:
      - curl
      - wget

# Install a package using a specific package provider
- action: package.install
  name: curl
  provider: apt

# Install a package specifying a repository
- action: package.install
  name: blox
  provider: homebrew
  repository: cueblox/tap
```

### Local package install support

Some package providers allow for installing a package from the local file system. It requires that the `file` property be set to `true`.

Providers supporting local install:

- apt / aptitude (Ubuntu/Debian)

#### Example

```yaml
- action: package.install
  name: /some/path/to/file/nano_8.1_amd64.deb
  file: true
```

## package.repository

| Key      | Type          | Optional | Description                                     |
| -------- | ------------- | -------- | ----------------------------------------------- |
| name     | string        | no       | Alias of url                                    |
| key      | RepositoryKey | yes      | See table below                                 |
| provider | string        | yes      | Default value provided, specify package manager |

### RepositoryKey

| Key         | Type   | Optional | Description |
| ----------- | ------ | -------- | ----------- |
| url         | string | no       |             |
| name        | string | yes      |             |
| key         | string | yes      |             |
| fingerprint | string | yes      |             |

_More documentation to come_
