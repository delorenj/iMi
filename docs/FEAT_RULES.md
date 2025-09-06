# iMi Feat Command Rules

## Creating a Feature Worktree

> For This Example
> Consider a repo `delorenj/coolode` with this location: `/path/to/coolcode/trunk-main`
>
> - `REPOSITORY_PATH`: `/path/to/coolcode/trunk-main` (required, trunk branch)
> - `REPOSITORY_NAME`: `coolcode`
> - `IMI_PATH`: `/path/to/coolcode` (parent of trunk-main, and all other branches/worktrees)


**When run outside of a repository:**
- `--repo|-r` flag is required e.g. `iMi feat somefeature -r coolcode`

**When run in a repository** 
- it does all of the above
- it checks to ensure repo is registered (by the `iMi init`) and grabs the db data
- it checks the directory structure to ensure it adhere's to iMi's conventions
  - if either the above checks fail
    - it runs `iMi init`
    - it checks the directory structure to ensure it adhere's to iMi's conventions
      - if it still doesn't have the right structure
        - it exits with an error.
      - if it is good now
        - continue 
  - If both checks pass
    - it runs `git worktree add $IMI_PATH/feat-somefeature -B feat/somefeature origin/main --checkout`
    - `cd /path/to/coolcode/feat-somefeature`
      - if `/path/to/coolcode/feat-somefeature` doesn't exist
        - clean up potential errors (paths, worktrees)
        - exit with error
    - if `sync` enabled (and `/path/to/coolcode/sync` exists)
      - copy all the global and repo contents to the feature branches' root
    - register the new worktree with iMi by adding it to the worktrees table (might not exist yet, but now's a good time to make it!)
