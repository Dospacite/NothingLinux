# AUR submission bundle

This directory is the complete contents for the `nothing-linux` AUR package
repository. It builds the tagged source release with Cargo in the package
environment; it does not download or repackage a prebuilt binary.

Before publishing a new release, update `pkgver`, download the matching GitHub
tag archive, replace `sha256sums`, and regenerate `.SRCINFO`:

```sh
cd packaging/aur
makepkg --printsrcinfo > .SRCINFO
makepkg --verifysource
```

Copy `PKGBUILD` and `.SRCINFO` into the root of the separate AUR Git
repository, review the resulting diff, then push it. Do not commit built
packages, source archives, or the `src/` and `pkg/` directories.
