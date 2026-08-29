# Project Archon

## AI Usage
All of the react side used AI Guidance. I do not know why but the Antigravity-CLI counted towards my AI coding hours even when it was open in the background. I think it might have been counting my app as my app opens an antigravity-cli instance. All AI Usage has more detail in the commit messages. I used a lot of premade components most of my work was just wiring everything together to make it work.

## Install
To install this app you go to the releases page and download the installer. Do __not__ download the release marked as Non-User as that is what the installer fetches. If you are an advanced user you can download the application itself and deal with any issues caused by directory. Then you run the app with arguments detailing in the directory the app has been installed to.

### Arguments:
#### Server:
```
./talos server
```
#### Client:
```
./talos client <Server IP Address>
```

E.g.
```
./talos client 192.168.68.123
```

## Usage
The server prints out the url at which the server can be accessed.

##  AI Guidance
For me I consider AI guidance as when the AI does researching and returns a list of modules and docs I can use for my code allowing me to spend less untracked time researching.

## Build from Source
The instructions to build this entire app from source are as follows. The user must be familiar with using git and cargo. Rust must be installed and documentation on how can be found at: ![Rust Lang Install Docs](https://rust-lang.org/tools/install/) . Then you must follow everything it says to install it. Then you must also install git which can be found at ![Git Install](https://git-scm.com/install/) . After all of this you must run:
```
git clone https://github.com/NMaster23/Project-Archon.git
```
And then go into that directory. Then using cargo you run it using:
```
cargo run -p talos
```
or you can build it using:
```
cargo build -p talos
```

For the installer the commands are:
```
cargo run -p installer
```
and:
```
cargo build -p installer
```