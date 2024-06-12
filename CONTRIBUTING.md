# FE2O3 Rust AMQP Library Contributing Guide

Thank you for your interest in contributing to the FE2O3 (Iron Oxide) library.

- For reporting bugs, requesting features, or asking for support, please file an issue in the [issues](https://github.com/minghuaw/fe2o3_amqp/issues) section of the project.

- To make code changes, or contribute something new, please follow the [GitHub Forks / Pull requests model](https://docs.github.com/articles/fork-a-repo/): Fork the repo, make the change and propose it back by submitting a pull request.

## Pull Requests

- **DO** submit all code changes via pull requests (PRs) rather than through a direct commit. PRs will be reviewed and potentially merged by the repo maintainers after a peer review that includes at least one maintainer.
- **DO** review your own PR to make sure there aren't any unintended changes or commits before submitting it.
- **DO NOT** submit "work in progress" PRs. A PR should only be submitted when it is considered ready for review and subsequent merging by the contributor.
  - If the change is work-in-progress or an experiment, **DO** start it off as a temporary draft PR.
- **DO** give PRs short-but-descriptive names (e.g. "Improve code coverage for Azure.Core by 10%", not "Fix #1234") and add a description which explains why the change is being made.
- **DO** refer to any relevant issues, and include [keywords](https://docs.github.com/articles/closing-issues-via-commit-messages/) that automatically close issues when the PR is merged.
- **DO** tag any users that should know about and/or review the change.
- **DO** ensure each commit successfully builds. The entire PR must pass all tests in the Continuous Integration (CI) system before it'll be merged.
- **DO** address PR feedback in an additional commit(s) rather than amending the existing commits, and only rebase/squash them when necessary. This makes it easier for reviewers to track changes.
- **DO** assume that ["Squash and Merge"](https://github.com/blog/2141-squash-your-commits) will be used to merge your commit unless you request otherwise in the PR.
- **DO NOT** mix independent, unrelated changes in one PR. Separate real product/test code changes from larger code formatting/dead code removal changes. Separate unrelated fixes into separate PRs, especially if they are in different modules or files that otherwise wouldn't be changed.
- **DO** comment your code focusing on "why", where necessary. Otherwise, aim to keep it self-documenting with appropriate names and style.
- **DO** make sure there are no typos or spelling errors, especially in user-facing documentation.
- **DO** verify if your changes have impact elsewhere. For instance, do you need to update other docs or exiting markdown files that might be impacted?
- **DO** add relevant unit tests to ensure CI will catch future regressions.

## Merging Pull Requests (for project contributors with write access)

- **DO** use ["Squash and Merge"](https://github.com/blog/2141-squash-your-commits) by default for individual contributions unless requested by the PR author.
  Do so, even if the PR contains only one commit. It creates a simpler history than "Create a Merge Commit".
  Reasons that PR authors may request "Merge and Commit" may include (but are not limited to):

  - The change is easier to understand as a series of focused commits. Each commit in the series must be buildable so as not to break `git bisect`.
  - Contributor is using an e-mail address other than the primary GitHub address and wants that preserved in the history. Contributor must be willing to squash
    the commits manually before acceptance.

## Developer Guide

### AMQP 1.0 Protocol

- Core [standard](http://docs.oasis-open.org/amqp/core/v1.0/os/amqp-core-overview-v1.0-os.html).
- WebSocket
  binding [working draft](http://docs.oasis-open.org/amqp-bindmap/amqp-wsb/v1.0/csprd01/amqp-wsb-v1.0-csprd01.html)
- Management extension [working draft](https://groups.oasis-open.org/higherlogic/ws/public/document?document_id=54441)
- Claims based security [working draft](https://groups.oasis-open.org/higherlogic/ws/public/document?document_id=62097)
- [Extensions](https://www.amqp.org/specification/1.0)

### Full Local Setup

#### Pre-requisites

`rust` stable toolchain newer than 1.75.0 is required to build the project, and the nightly
toolchain is required for building the documentation locally. Please follow the instruction on the
[Rust website](https://www.rust-lang.org/tools/install) to install the toolchain. `cargo` is
included in the installation.

`cargo make` is used in some crates to test with different features. Install it with:

```bash
cargo install cargo-make
```

Docker is required for running integration tests in `fe2o3-amqp`. Please follow the instruction on
the [Docker website](https://docs.docker.com/get-docker/) to install Docker.

### Building the project

The project is divided into multiple crates. Each crate should be built separately. The main crate
is `fe2o3_amqp`. Build each crate with:

```bash
cd <crate-name>
cargo build
```

### Testing the project


`cargo make` is used for testing combination of different features in some crates. For crates that
do not have `Makefile.toml`, `cargo make test` is an alias to `cargo test`. Additionally, Docker
is required for `fe2o3-amqp` to run integration tests.

Test each crate with:

```bash
cd <crate-name>
cargo make test
```

Run the following to check if all features in `fe2o3-amqp` are gated properly:

```bash
cd fe2o3-amqp
cargo make feature_check
```

Run the following to check if all examples build:

```bash
cd examples
cargo make check
```

Test if the documentation build on docs.rs with the command below. Please note that this command
requires the nightly toolchain.

```bash
cd <crate-name>
cargo make docsrs
```
