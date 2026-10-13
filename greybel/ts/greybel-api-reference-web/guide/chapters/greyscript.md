# GreyScript programs

A Grey Hack program starts as source compiled in the CodeEditor or through [build](#SHELL_BUILD), then runs from a terminal under a program name with optional arguments. Each source file can contain at most 160,000 characters. Greybel's build tools can prepare source and deploy it to the game; see [Tools](#guide-tools).

## Arguments

`params` is a list containing the program's command-line arguments as strings. [program_path](#GENERAL_PROGRAM_PATH) returns the running program's path, and [current_path](#GENERAL_CURRENT_PATH) returns the current working directory.

```
print program_path.len > 0 // 1
name = "user"
if params.len > 0 then name = params[0]
if name == "--help" then
    print "Usage: myprogram [name]"
    exit
end if
print "Hello, " + name // Hello, user
```

[exit](#GENERAL_EXIT) terminates the program immediately. It accepts an optional message. Convert numeric arguments explicitly with [val](#GENERAL_VAL) and validate them before use.

## Input and timing

[user_input](#GENERAL_USER_INPUT) displays an optional prompt and waits for terminal input. Its options support password entry, single-key input, and history behavior. The final `echo` argument defaults to `1`; pass `0` to hide typed input. Invalid argument types return `user_input: Invalid arguments`.

```
name = user_input("Your name: ")
print "Hello, " + name
```

[wait](#GENERAL_WAIT) pauses execution for the specified number of seconds, or one second when omitted. The duration must be from `0.01` to `300` seconds; anything else is a runtime error. [time](#GENERAL_TIME) returns the seconds since the program started, so the difference of two readings measures an operation.

```
print "starting"
started = time
wait 0.5
print time - started >= 0.5 // 1
```

[yield](#GENERAL_YIELD) yields execution until the host's next execution cycle. Use host-provided timing functions rather than busy loops when a delay is required.

## Terminal output

The game doesn't send each print to the terminal on its own. Prints collect in a batch, which the terminal is sent every 200 ms the program runs, when the program waits ([wait](#GENERAL_WAIT), [user_input](#GENERAL_USER_INPUT), or a transfer, decipher or scan of the game), and when it ends. A program it launches prints into the same batch.

`print(text, true)` first sends the batch so far, then starts one that replaces everything the terminal shows: the replacing line and every line printed after it, up to the next send. A progress display therefore needs a [wait](#GENERAL_WAIT) after each step; without it, all steps land in one batch and the terminal shows them all.

The third argument, `parse_newlines`, defaults to `1` and turns each literal `\n` (backslash followed by `n`) into a line break. Pass `0` to print those characters as written. `print("")` prints an empty line.

```
for step in range(1, 3)
    print("Progress: " + step + "/3", true)
    wait(0.2)
end for
```

[clear_screen](#GENERAL_CLEAR_SCREEN) clears the terminal and drops what the batch holds, so a line printed just before it never shows.

[print](#GENERAL_PRINT), [exit](#GENERAL_EXIT) and the prompt of [user_input](#GENERAL_USER_INPUT) accept TextMeshPro rich-text tags such as `<color=red>`, `<b>` and `<size=80%>`. [format_columns](#GENERAL_FORMAT_COLUMNS) aligns text in columns by its characters, so tags inside would break the alignment: format the plain text first and add the tags afterwards.

```
table = format_columns("NAME SCORE" + char(10) + "ada 42")
print "<b>" + table + "</b>"
```

## Launching programs

[launch](#SHELL_LAUNCH) runs another program and waits until it ends. It returns `1` once the program started, whether it then completes or fails, and `0` or an error message when it could not start, for example when its file is missing or isn't executable. On a remote shell, the program starts in that shell user's home folder. A program launched more than 32 levels deep is interrupted with "Too many stack calls."

A launched program sees its own path as [program_path](#GENERAL_PROGRAM_PATH) and the path of the program that launched it as [launch_path](#GENERAL_LAUNCH_PATH); a program started from the terminal sees its own path in both. Values pass between them through [get_custom_object](#GENERAL_GET_CUSTOM_OBJECT), a map shared by a program and every program it launches. [Errors and pitfalls](#guide-errors) shows how to use this to run code that may fail.

## Imports

[import_code](#GENERAL_IMPORT_CODE) incorporates another source file when the program is compiled. It allows a program to span files while respecting the 160,000-character limit for each source file.

```
import_code("lib/tools.src")
print shout("hello") // HELLO
```

For this example, `lib/tools.src` defines `shout` as a function returning `text.upper`.

In Grey Hack, the import path is absolute and an imported file cannot import another file. The game finds `import_code(` anywhere in a file's text when it compiles, including in comments and strings: `// import_code("/lib/old.src")` still imports the file. Only an `import_code` written directly after `//`, as in `//import_code("/lib/old.src")`, is skipped. Greybel ignores both; its language server warns about each `import_code(` the game would still import. Greybel additionally resolves a relative path from the importing file, and an absolute path from the project folder: the folder of `greybel.json`; without one, the workspace folder in VS Code, or else the entry file's folder. During a build, Greybel rewrites paths for the files' in-game locations and moves nested imports into the entry file as required.

An `import_code` inside an imported file stays in the built program as an ordinary call, which fails with `Undefined Identifier: 'import_code' is unknown in this context` when it runs. In a runtime error of a program built with imports, an imported line is named by its file's name and its line in that file, as in `[lib.src line 2]`, and a line of the program's own file by its line alone, as in `[line 3]`.

Unlike Greybel's `#import`, `import_code` does not return a `module.exports` object. Imported definitions become available in the importing program's code. See [Greybel modules](#guide-greybel) for the build-time alternative.

## Game access

[get_shell](#GENERAL_GET_SHELL) returns the shell in which the program runs, or another accessible shell when credentials are supplied. The shell's [host_computer](#SHELL_HOST_COMPUTER) exposes files, processes, users, and networking operations. Use [typeof](#GENERAL_TYPEOF) to identify game objects and [isa](#guide-type-conversion) for language-level type checks.

```
shell = get_shell
computer = shell.host_computer
print typeof(computer) // computer
home = computer.File("/home")
print home.path        // /home
```

Game objects compare and hash by identity. Asking for the same underlying shell, computer, file, router, port, or library again returns the same object.

File operations require execute permission on every parent folder, not only access to the final file or folder. Changing directory with [cd](#GENERAL_CD) requires execute permission on the target folder as well. Partial recursive copies and deletions can return warnings instead of succeeding silently.

## MetaMail

[mail_login](#GENERAL_MAIL_LOGIN) returns a [MetaMail](#METAMAIL) account object. [read](#METAMAIL_READ) and [delete](#METAMAIL_DELETE) take lists of mail ID strings; `read` returns one content string per ID, while `delete` removes every match. [delete_all](#METAMAIL_DELETE_ALL) removes the whole account's mail. Fetching, reading, and deleting refresh the account first.

Every MetaMail method shares a two-second cooldown after the previous mail operation. Calling again sooner returns `MetaMail: Wait 2 seconds before using mail again.` Sending uses the account represented by the object as the sender.

The mail account of an NPC works like any other account, and its mailbox is filled the first time the account is used. Once an NPC is fired or has died, [mail_login](#GENERAL_MAIL_LOGIN) returns `This account is disabled.` In Grey Hack this can lag: the game checks a copy of the NPC's state that it keeps with the NPC's workstation, and that copy is updated only when the workstation is next accessed, for example by connecting to it. Until then a fired or dead NPC's account may still accept logins. Greybel reads the NPC's current state, so it refuses the login at once.

The API reference documents each object's methods, properties, return types, and possible error strings. Check permissions and return values instead of assuming that every operation succeeds.
