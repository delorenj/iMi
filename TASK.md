# TASK

Implement the `clone` command.

## Specification

The `clone` command should create a copy of a given repository from a remote source to the local machine. It should function identically to the `git clone` command, with the following acceptions:

- It should accept only a single argument: the name of the repository to clone
- If `user` is left out and only `name` is provided, it should default to "delorenj"
- The arg can be provided in 3 formats:
  - `name`
  - `user/name`
  - `https://github.com/user/name.git`
- The repo should be cloned into a directory named after the repository, in the `iMi System Path` (set in the `~/.config/iMi/config.toml`)
  - e.g., if cloning `repo-name`, it should create a directory called `repo-name` in the iMi System Path and clone the repo there with the name `trunk-main`
  - In this case (this server), the full path would be `/home/delorenj/code/repo-name/trunk-main`
  - If the target directory already exists then there are two possibilities:
    1. The repo is already an iMi repo, so instead of cloning, it should just `igo` to the directory and log a message indicating that the repo already exists and that it is switching to it.
    2. The repo is not an iMi repo (hasn't been initialized with iMi), so it should log a message indicating that the target directory already exists but it is not an iMi repo and it will convert it into one by initializing it with iMi after cloning. There is a script that does this `/home/delorenj/.config/zshyzsh/scripts/imify.py`. Ideally this script should be implemented as an iMi command in the future, but for now it can be called directly.
    - The script safely rearranged the existing contents of the directory into the iMi structure and then runs `iMi init`
    - `repo-name/` becomes the `repo-name/trunk-main/` directory.
