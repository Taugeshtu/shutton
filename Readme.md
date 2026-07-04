> for those of us who _don't_ live in terminal (yet)

## why shutton

I am a visual person, it is easier for me to navigate the digital side of my life in Thunar. I also use scripts. I want BUTTONS to launch my scripts!

## how shutton

minimalist Rust app that presents a text input, runs a shell command, shows/copies/filedrops output. Persists into itself, and can therefore be copy-pasted throughout your system and configured to do many useful things

## Usage

{quick-jump (not overwhelming) instructions how to use}

---

## Install

dependencies: 
- [rust installed in your system](https://rust-lang.org/tools/install/)
- `GTK4` system libraries. On Fedora:
```sh
sudo dnf install gtk4-devel
```
_(have instructions for your repo? happy to add - make an issue with them!)_

build & install with cargo:
```bash
cargo install --git https://github.com/Taugeshtu/shutton --root ~/.local
```

_Alternatively:_
```sh
# navigate to where you want it to live, for example, ~/Applications/Gits
git clone https://github.com/Taugeshtu/shutton
cd shutton
cargo install --path . --root .
```

## Version history

future
- [ ] work with needing sudo, I guess?
- [ ] figure out what we do if we have a shell script that needs extra input..

v0.4.0
- [x] persistence into the binary itself upon running

v0.3.0
- [x] additional arguments fields

v0.2.0
- [x] log action buttons: view, copy, drop as file
- [x] quit-on-done toggle

v0.1.0
- [x] present GUI, run shell script
- [x] run shell script when hitting "Enter"
- [x] quiet-quit when hitting "Esc"
