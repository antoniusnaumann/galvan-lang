- Add "todo" as special handling function
- Add "panic" and "todo" in
- Rework Ownership system to differentiate between exclusively owned (e.g., owned function return type) and shared owned (e.g., owned variables that need to be cloned to avoid a move)
- Add "MutOwned" to ownerships, this requires cloning instead of re-borrowing

