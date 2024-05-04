# Note on `ci.svg`

`ci.svg` is generated from `ci.mmd` using [the mermaid cli](https://github.com/mermaid-js/mermaid-cli).

`ci.mdd` is edited in [the mermaid live editor](https://mermaid.live/edit).

To regenerate `ci.svg`, make sure you have a local installation of `npm/npx`; then simply run:

```sh
npx @mermaid-js/mermaid-cli -i ./ci.mmd -o ./ci.svg
```