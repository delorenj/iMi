# TASK

Thank you for providing the code. After a thorough review of `cli.rs`, `worktree.rs`, and your `README.md`, I can confidently say that **you are already using the correct and most robust approach** for 99% of your application's logic.

The confusion arises from a single, special command: `go`. Let's break it down.

### Your Worktree Creation Logic is Excellent

First, to be clear, your initial concern about needing to `cd` before creating a worktree is something you have already solved correctly.

In your `worktree.rs` file, the `create_worktree_internal` function is a perfect example of how this should be done. You are not changing the current directory of your `imi` process. Instead, you are:

1.  **Detecting Context**: Finding the root of the git repository from the current directory (`env::current_dir()`).
2.  **Determining Parent Path**: Using your `detect_imi_path` helper to find the correct parent directory where all sibling worktrees should reside. This is the key step that decouples your app from the CWD.
3.  **Constructing an Absolute Path**: Building the full, explicit path for the new worktree (`let worktree_path = imi_path.join(worktree_name)`).
4.  **Executing the Command**: Passing this explicit path to your Git manager to create the worktree.

This is the ideal implementation. It's robust, not dependent on where the user is, and avoids the process isolation problem entirely for actions like creating, removing, or managing files.

### The `go` Command: The Special Case

The fundamental issue we first discussed—that a child process cannot change its parent's (the shell's) directory—applies specifically and _only_ to your `Go` command.

Your `cli.rs` defines its purpose clearly: "**Navigate** to a worktree or repository". The user's intent is to have their shell's prompt end up in a new directory.

Your `fuzzy_navigate` function already does all the hard work of finding and returning the correct `PathBuf` for the destination. You are 95% of the way to the complete solution.

### The Final Step: Connecting Your App to the Shell

To make the `go` command work as intended, you must implement the two-part solution we discussed earlier: have your Rust binary output the path, and have a shell function capture it.

**1. Modify Your Rust Binary's `main` Function**

When your application handles the `Commands::Go`, your code should call `fuzzy_navigate` and, upon success, **print the resulting path to standard output (`stdout`) and nothing else.**

Here is a conceptual example of what the logic in your `main.rs` should look like:

```rust
// In your main function, when handling the CLI commands...
match cli.command {
    Some(Commands::Go { query, repo, worktrees_only, include_inactive }) => {
        // Assume 'worktree_manager' is your initialized WorktreeManager
        let result = worktree_manager.fuzzy_navigate(
            query.as_deref(),
            repo.as_deref(),
            worktrees_only,
            include_inactive,
        ).await;

        match result {
            Ok(path) => {
                // This is the critical change:
                // Print the canonicalized path directly to stdout.
                // The shell function will capture this output.
                print!("{}", path.display());
            }
            Err(e) => {
                // Print errors to stderr to avoid polluting stdout.
                eprintln!("Error: {}", e);
                // Exit with a non-zero status code to signal failure.
                std::process::exit(1);
            }
        }
    }
    // ... handle other commands like Feat, Review, etc. normally ...
}
```

**2. Define a Shell Wrapper Function**

The second and final piece is to create a shell function or alias that the user will interact with. This function will call your `imi go` command, capture the path it prints, and then execute the _real_ `cd` command.

Instruct your users to add the following to their `~/.zshrc` or `~/.bashrc`:

```bash
# A shell function to wrap 'imi go' for navigation
go() {
  # Execute the imi binary and capture its stdout into a variable.
  # Stderr will pass through to the console.
  local target_dir
  target_dir=$(imi go "$@")

  # Check if the command was successful (exited 0) and produced output.
  if [ $? -eq 0 ] && [ -n "$target_dir" ]; then
    # Use the shell's BUILTIN 'cd' command to change the directory.
    cd "$target_dir"
  fi
}
```

With this in place, the user's workflow is seamless:

1.  User types `go my-feature-worktree`.
2.  The shell executes the `go()` function.
3.  The function runs `imi go my-feature-worktree`.
4.  Your Rust app calculates the full path and prints `/home/user/code/my-project/feat-my-feature-worktree` to stdout.
5.  The `target_dir` variable captures this string.
6.  The shell runs `cd /home/user/code/my-project/feat-my-feature-worktree`, and the user's prompt changes.

You have built a very clean and powerful tool. This final piece of shell integration for your `go` command is the standard, accepted pattern for solving this classic CLI challenge.
