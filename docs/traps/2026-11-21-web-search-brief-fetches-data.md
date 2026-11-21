# A brief that permits web search lets an agent fetch data files

Found by the Head of Economics (2026-11-21). A brief that said "use web search" let a research agent download four
FRED series into its local `data/raw/`. The download did not go through the Head of Technology, and the files
were not in the data store, so an auditor could not reproduce the note.

- Rule for a brief: write "pages only, no data files" when an agent may search the web.
- A data file that a document cites goes into the store first (a request to the Head of Technology). The document
  cites the store path.
- Related: some `fa` commands also download a missing file (ticket `T-20261121-cache-only-switch`).
