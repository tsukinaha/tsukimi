# Contribution guidelines

## Translations

If you'd like to help translating **Tsukimi** into your language, please head over to [Weblate](https://hosted.weblate.org/projects/tsukimi/).

<a href="https://hosted.weblate.org/engage/tsukimi/">
    <img src="https://hosted.weblate.org/widget/tsukimi/main/multi-auto.svg" alt="Translate Status" />
</a>

## Your first contribution

### Dependencies

Prefer your distribution's packaged `tsukimi` build when it is available. Distro
packages should already provide correct runtime dependencies and resource
installation.

If you want to build Tsukimi from source, see `meson.build` for dependency
requirements. Tsukimi needs relatively new GTK4 and libadwaita releases.

### Development

The repository includes a `justfile` with the common Meson workflows:

```sh
just setup
just build
```

`just setup` configures the local `build/` directory with a development install
prefix under `build/dev-prefix/`. `just build` compiles the GUI program through
Meson.

To run the GUI from the build directory:

```sh
just run
```

The `run` recipe installs into `build/dev-prefix/` first, then launches
`build/src/tsukimi` with `GSETTINGS_SCHEMA_DIR` and `XDG_DATA_DIRS` pointed at
that local install tree so schemas, icons, and bundled resources are found.

To specify debug/release build, run:

```sh
meson setup build --prefix=/usr -Drust-target={debug,release}
meson compile -C build
```

## AI Contribution Policy

Tsukimi is a project by humans for humans. We prefer contributions that
are produced by human creativity, we expect a human to take full
responsibility for each contribution, and we will take more joy in
reviewing contributions when there's people at the other end of the
line to stand by their changes.

If you use LLM/GenAI tools for your contributions, here are the rules
you must follow:

### Requirements

1. Use AI as a tool. Verify behavior, correctness, and compatibility
   yourself prior to submitting your contribution. Do not ask the
   maintainers to do this for you.
1. Keep changes narrow and limited. Do **NOT** use LLM/GenAI tools to
   generate broad rewrites, large refactorings, or style changes.
1. Do **NOT** submit generated code, documentation, or tests that you
   don’t understand.
1. Do **NOT** fabricate benchmarks, bug reports, test results, code
   samples, or reproducers.
1. Do **NOT** include private code, credentials, tokens, or any other
   confidential material.
1. Do **NOT** feed the review feedback to an LLM/GenAI tool.
1. Respect the licensing and attribution requirements.


### Disclosure

Always disclose the use of LLM/GenAI tools when creating an issue or
a merge request. Do not include trailers like “Co-authored-by:” or
“Assisted-by:” in commit messages, since they serve as free advertising
for AI companies.


<sup>
    Some of the content is copied from <a href="https://gitlab.gnome.org/GNOME/gtk">GTK Project</a>
</sup>
