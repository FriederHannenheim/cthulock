# Configuration
Place a style.slint file in `$XDG_CONFIG_HOME/cthulock` or `$HOME/.config/cthulock`. Your Slint component always needs the following:
```slint
// will be set when the password is currently being checked
// you can use this to disable a text box for example
in property<bool> checking_password;
in-out property<string> password <=> password.text;
// Submit password callback. When this is called the PAM authentication process is called with the password you have provided.
callback submit <=> password.accepted;

// A LineEdit which the user enters the password in
password := LineEdit {}
```
These are optional properties that can also be used
```slint
// It is reccommended to add the following line so that the user can start typing immediately and does not need to focus the password field explicitly
forward-focus: password;

// A clock string will be available using this property. 
// TODO: The user should be able to switch between 12 and 24 hour clock. Currently it is always 24 hour
in property<string> clock_text;
```

When testing your configuration run cthulock with `--no-fallback`, otherwhise a fallback lockscreen is shown to ensure your screen is locked even if the configuration is invalid.

Before using a configuration to lock your screen you should test if it works in a nested Wayland session so that you don't get locked out of your PC. Cthulock will check if the configuration works and if all neccesary properties and callbacks exist before locking the screen but you can always do something like disabling the password input which will lock you out. That cannot be checked for.

Testing is best done using labwc:
```
$ labwc -s "cthulock --no-fallback"
```
