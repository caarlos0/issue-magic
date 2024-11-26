# issue-magic

Auto-label GitHub issues using AI and a rule file.

## How it works?

You can create a `config.toml` like so:

```toml
[repository]
owner = "caarlos0"
name = "issue-magic"

[labels]
[labels.enhancement]
condition = "is asking for a new feature"

[labels.bug]
condition = "is describing a bug or malfunction"

[labels.question]
condition = "is an user question about how to use the project"

[labels.documentation]
condition = "is about a problem with documentation, readme, or examples"
```

And then run `issue-magic`.

It will ask you to confirm its choices. That can be disabled with the flag
`--auto`.

For this to work, you'll also need to have the `GITHUB_TOKEN` and
`ANTHROPIC_API_KEY` environment variables set.

## Status

Very early alpha.
