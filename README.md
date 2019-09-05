# git-credential-null

This [git credential helper][helper] disables any known git password prompt,
and it's meant to be used on automated programs that want to get an error
instead of an hanged git prompt when a repository requires authentication.

This helper is released under the MIT license.

## Usage

Install this Rust package with:

```
cargo install git-credential-null
```

Then pass the `-c credential.helper=null` when you want to disable the prompt.
For example:

```
git -c credential.helper=null clone https://github.com/ghost/doesnt-exist
```

[helper]: https://git-scm.com/docs/gitcredentials#Documentation/gitcredentials.txt-helper
