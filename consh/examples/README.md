# ConSh examples

Short scripts that show what ConSh does better than Bash. Each `NAME.sh` is a ConSh script and `NAME.bash` does the same in Bash, with the extra code or the pitfall that Bash needs. See [FOR-BASH-USERS.md](../FOR-BASH-USERS.md) for the language.

Run them from this directory:

```
../bin/consh spaces.sh
bash spaces.bash
```

| Example | Shows |
|---|---|
| [spaces.sh](spaces.sh) / [spaces.bash](spaces.bash) | Renaming files whose names contain spaces. ConSh never splits a variable, so no quotes are needed; the Bash version shows what an unquoted `$f` does. |
| [logsummary.sh](logsummary.sh) / [logsummary.bash](logsummary.bash) | Summarizing [server.log](server.log). A function takes two lists as two arguments, and `> (first . rest)` splits output into variables. Bash keeps the lines in strings, splits them again with `read`, and needs `head`, `tail` and an array for the rest. |
| [lazy.sh](lazy.sh) / [lazy.bash](lazy.bash) | Searching `$(seq 1000000000)` and an endless list made by a function. `$( )` is read lazily, and the variable set in the loop survives. In Bash, `$(seq 1000000000)` reads everything first, a pipe into `while` loses the variable, and the loop needs process substitution. |
| [typo.sh](typo.sh) / [typo.bash](typo.bash) | A misspelled variable name. ConSh stops with the file, line and column; Bash expands it to an empty string and would run `rm -rf /*`. (The scripts only `echo` the command.) |

The expected output of each script is in [../test/examples](../test/examples), and `test/run_all.sh` checks it. `typo.sh` ends with status 1 on purpose.
