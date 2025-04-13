<div align="center">

# mane - make a new elephant 🐘

A copy & replace tool that allows you to reuse code without using templates. 

<p align="center">
  <img src="https://placehold.co/200" alt="mane logo" width="200"/><br/>
  <em><b>mane</b> means imitation in Japanese.</em>
</p>

</div>

---

`mane` features are the following:

- copy files excluding files that have been gitignored
- replace contents and file names wisely

## INSTALLATION

TODO

## SYNOPSIS

```bash
# Copy Awesome/foo to Cool/ and replace all "foo" with "bar"
$ mane -c Awesome/foo Cool/ -r foo bar

# =>
# Before:
# Awesome/
#   foo/index.tsx
#   foo/sub/foo-item.tsx

# After:
# Cool/
#   bar/index.tsx
#   bar/sub/bar-item.tsx

# Copy multiple sources to a single target
$ mane -c Awesome/foo Awesome/bar Cool/ -r foo bar
```

## COMMAND LINE OPTIONS

| Option | Description |
| --------|-------------|
| `-c, --copy SOURCE [SOURCE...] TARGET` | Copy files or directories to a single target |
| `-r, --replace FROM TO` | Replace text (multiple allowed) |
| `-i, --in-place` | Replace file/directory names |
| `--include-git-ignore` | Include .gitignored files |
| `-v, --version` | Show version |
| `-h, --help` | Show help |

### -c, --copy SOURCE [SOURCE...] TARGET

```bash
$ mane -c ./foo.txt ./bar.txt ./target/
```

### -r, --replace FROM TO

```bash
# Replace in stdin
$ echo "Hello, World" | mane -r Hello Hi
Hi, World

# Multiple replacements
$ echo "Hello, World" | mane -r Hello Hi -r World Japan
Hi, Japan

# Replace in files
$ mane -r hello hi foo.txt
```

`mane` handles different case styles. The following chart is replaceing `HelloWorld` with `GoodMorning`:

| Case           | Original Format | Converted Result |
|----------------|-----------------|------------------|
| Pascal         | HelloWorld      | GoodMorning      |
| Kebab          | hello-world     | good-morning     |
| Camel          | helloWorld      | goodMorning      |
| ScreamingSnake | HELLO_WORLD     | GOOD_MORNING     |
| Snake          | hello_world     | good_morning     |

