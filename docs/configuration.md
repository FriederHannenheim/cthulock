# Configuration
Your Slint component always needs the following:
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