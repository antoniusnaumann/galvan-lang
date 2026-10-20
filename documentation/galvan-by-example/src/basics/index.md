# Basics

This chapter covers the building blocks that every other example relies on:
variables, the primitive types, operators, string and character values, and
the newline rules that make semicolons unnecessary.

Galvan's guiding principle shows up early: **values by default**. Bindings are
immutable unless marked `mut`, assignment copies rather than moves, and
sharing state is a deliberate, visible choice (`ref` — covered in the
[Ownership](../ownership/index.md) chapter).
