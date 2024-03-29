# Cthulock
Cthulock is a screen locker for Wayland focused on customizability. You can style your lock screen using the [Slint](https://slint.dev/docs.html) language. An example config is already provided for you to build upon.

Cthulock is still in development, if you have any suggestions for features please open an issue. Code contributions through pull requests are also always welcome.

## Example Screenshot
![Example Screenshot](./docs/example_config_screenshot.png)
## Installation
#### Compile dependencies
- rust
- clang
- xkbcommon
- egl
- pam

If you have the repo cloned you can just do
```
$ cargo install --path .
```

Alternatively you can install from git like this
```
$ cargo install --git https://github.com/FriederHannenheim/cthulock.git
```

## Running
#### Runtime dependencies
- Wayland compositor supporting ext-session-lock-v1
- OpenGL

Just run cthulock without any parameters
```
$ cthulock
```

## Configuration
Copy the sample config into `$XDG_CONFIG_HOME/cthulock` and start customizing it, for example by swapping out the wallpaper or moving the clock anywhere else. You can find the Slint syntax [here](https://slint.dev/docs.html).

For details on which properties and callbacks your Slint component needs to have look at the [configuration docs](./docs/configuration.md)

## Credits
The wallpaper in the example configuration is "[Wallpaper](https://www.flickr.com/photos/131042142@N05/16252364850)" by [DyosEL](https://www.flickr.com/photos/131042142@N05) and is licensed under CC BY 2.0. 
