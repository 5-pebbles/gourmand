# Miros 🌸🌿

This is an experimental dynamic linker built to document the dynamic linking process and replace ld.so on my systems.


## Contributing Rules:

Any contributions you make will be greatly appreciated; however there are some rules:

1. **Avoid Closures:** I don't know why but sometimes big closures cause segfaults.
2. **Avoid Unwraps:** They raise a runtime error; something about global allocators and thread local storage.

