# Imperator army UI reference

`imperator-army-tactics.png` is the original, unmodified 1920×1080 screenshot
published by Paradox Interactive in the [25 March 2019 development diary](https://forum.paradoxplaza.com/forum/threads/imperator-development-diary-25th-of-march-2019.1162743/).
Source: https://forumcontent.paradoxplaza.com/public/452776/overview2.png

The province Military tab follows its compact force summary, composition cards,
horizontal cohort-preference controls, tactic picker, and red/gold cohort ledger.
`build.rs` extracts the five original animal tactic symbols from this reference,
without regenerating or redrawing them. A generated owl identifies Augustus's
additional Balanced tactic; Imperator does not provide that tactic. See
`../generated-balanced-tactic-icon.md`.

Recruitment, terrain frontage, tactic counters and cohort capabilities retain the
Augustus rules in `AUGUSTUS-game-design.md`. Primary/front and secondary/rear
preferences choose the initial center and its replacements. Ranged and siege
support deployment remains automatic. Flank width is 1 through 5 per side, subject
to terrain frontage.

Reference artwork belongs to Paradox Interactive. It is separate from the generated
Augustus unit artwork documented in `generated-art-military.md`.
