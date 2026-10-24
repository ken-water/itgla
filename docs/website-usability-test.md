# ITGLA website usability validation

Status: protocol ready; **no participant sessions have been conducted**. Local website iteration, October 24, 2026. Do not describe the interface as user-validated based on browser automation.

## What we need to learn

Can an IT operator or independent developer understand the product, distinguish maintained inventory from automatic discovery, and choose the right desktop download without thinking an account is required?

Use five initial participants: three people who maintain servers/services and two independent developers who manage their own deployments. Avoid relying only on people who already know ITGLA. This is a qualitative first round, not a statistically reliable conversion experiment.

## Session setup

- Use the same local version and record the Git commit plus whether the working tree is dirty. When published later, record the deployed version and URL instead.
- Ask which operating system and processor the person uses before the tasks. Include Windows, Apple silicon and Intel Mac choices across sessions when feasible.
- Explain that the website is being tested. Ask permission before recording. Use fictional infrastructure and no real credentials.
- Do not offer product explanations, point to controls or reveal the expected answer during a task. Record help as a task failure, then assist after observation.
- Stop before actually downloading or executing software unless the participant separately agrees. Selecting the correct package is enough for this study.
- Do not send recruitment invitations, record people or add website tracking without authorization. This document is preparation only.

## Tasks and observations

| Task | Neutral prompt | Success evidence |
|---|---|---|
| First impression | View the top of the homepage for five seconds, then explain what the product does and who might use it. | Describes local infrastructure records and a relevant use case; does not assume automatic discovery. |
| Find a capability | You need to remember a service's version, port and hosting machine. Show where the website explains whether this tool can help. | Finds the real screenshot/workflow and explains the three details correctly. |
| Understand maintenance | How would information about a changed server get updated? | Understands that the records must be maintained manually. |
| Choose a package | Find the appropriate package for your computer and explain why you chose it. | Correct OS, CPU architecture and package type; recognizes the unsigned-package notice where applicable. |
| Verify before running | Show how you would check that this package matches the published release. | Finds the checksum file and instructions for their operating system. |
| Explain account/data boundary | What would you need to do before you can use the desktop app? Where would your inventory be stored? | No website account needed; local data; website sign-in does not enable desktop sync. |

After each task, ask: “What, if anything, made that unclear?” Finish with: “What would you need to see before trying this with a small fictional inventory?” Avoid asking whether the design is “world-class” or whether the person likes it.

## Record one row per participant and task

| Participant code | Role / OS / architecture | Task | Completed without help | Time | Incorrect expectation | Exact words | Severity / follow-up |
|---|---|---|---|---|---|---|---|
| Not yet run | | | | | | | |

Keep identifying details out of the results document. Store any consented recordings separately with an agreed deletion date.

## Decision rules for the first round

- Any wrong download choice, mistaken belief that an account is required, or assumption of automatic discovery triggers a copy/navigation review.
- Aim for at least four of five participants to explain local storage and manual maintenance without help. This is a diagnostic threshold, not a population estimate.
- Aim for all five to choose the correct package for their device. Investigate every failure before publishing further visual refinements.
- Separate product capability gaps from website comprehension failures. Keep verbatim evidence and the exact affected screen.
- Prioritize an issue that blocks a task over a preference about appearance. Re-test the changed task with new participants when possible.

## Validation still requiring people or devices

Native screen-reader review (including Windows Narrator or NVDA), Safari on macOS/iOS, actual mobile touch behavior, and these participant sessions remain open. Playwright WebKit on Windows is engine coverage, not proof of Safari behavior on Apple devices. No conversion improvement or customer preference is established by the local checks.
