# dflow

dflow is a tool designed to generate control flow and data flow representations from python source code. It parses the source code to build control flow graphs (CFGs) and uses the RVSDG IR (Regional Value State Dependency Graph) to enable data flow extraction.

## Features
- Parse Python source code to generate control flow graphs (CFGs).
- Support for inter-procedural analysis in CFGs (upcoming).
- Use RVSDG as an intermediate representation to ease data flow representation (in progress).
- Extract detailed data flow representations from RVSDG (planned).


Output will include the generated control flow graph, and when available, data flow visualizations.

## TODO
- [x] Parse Python source code and generate CFG.
- [ ] Implement inter-procedural calls for the CFG.
- [ ] Implement RVSDG IR on the control flow graph.
- [ ] Extract data flow representation from the RVSDG.
