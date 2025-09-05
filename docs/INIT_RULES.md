# iMi Init Command Rules

## Init Flow

**When run outside of a repository:**
- It creates a default configuration file if one does not exist or updates it if the `--force` flag is provided.
- It creates the local database if it does not exist or updates it if the `--force` flag is provided.
- [TBD LATER] If discovery is enabled, it registers itself with the 33GOD server (Jelmore) as an iMi-capable host.

**When run in a repository** 
- it does all of the above
- it checks the directory structure to ensure it adhere's to iMi's conventions
  - If it doesn't
    - it exits with an error.
  - If if does
    - it registers the repository in the database 
      - if that repo is already registered
        - it exits with an error
      - if it's not registered
        - it registers the iMi path with the database
          - the iMi path is the parent directory of all the branches (e.g. `repo` in `/path/to/repo/trunk-main`)
        - it creates a .iMi/ dir in the iMi path. This is kinda like a `.git/` dir. Contents TBD.

## Global iMi Config
- **Storage**: `~/.config/iMi/config.toml`
- **Scope**: System-wide, one per host

## Path Detection Logic

### When in Trunk Directory (`trunk-*`)
Consider a repo `delorenj/coolode` cloned with this command: `gh repo clone delorenj/coolcode /path/to/coolcode/trunk-main`

Here's the naming convention and terminology:

- REPOSITORY_PATH: /path/to/coolcode/trunk-main (implicitly, refers to the main branch)
- REPOSITORY_NAME: coolcode
- IMI_PATH: /path/to/coolcode (parent of trunk-main, and all other branches/worktrees)

