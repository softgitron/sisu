# 21 - Recruitment brief

This brief is the material for recruiting the outsiders that the
experiment in [18-next-steps.md](18-next-steps.md) needs. The rules it
applies are fixed in [20-pre-registration.md](20-pre-registration.md)
(participants in section 3, procedure in section 5); where the two differ,
20 governs. Every role is paid, because volunteers for an unreleased
language are not a plan (18). Selection is by background diversity under
published criteria. Choices that 18 and 20 do not make are marked
**[default proposed; owner to confirm]**, and things only the owner can
supply are written `[owner: ...]`. Week numbers are weeks of the plan;
week 1 begins on `[owner: date of week 1]`. The owner deferred every fee,
contracting and compensation item in October 2026; the brief is complete
apart from them and nothing is posted until they are filled.

## 1. The roles at a glance

| Role | Places | Paid time | Weeks | Fee | Section |
|---|---|---|---|---|---|
| Corpus author, non-Rust arm (Go, Kotlin or Swift) | 3 | about 4 weeks | 14-32 | `[owner: amount]` | 3 |
| Rust control-group member (corpus author, Rust arm) | 3 | about 4 weeks | 14-32 | `[owner: amount]` | 4 |
| Go port control (porter S1) | 1 | 5 weeks | 29-40 | `[owner: amount]` | 5 |
| Go/Java runtime reviewer | 1 | 2 weeks | 1-10 | `[owner: amount]` | 6 |
| Independent referee | 1 | about 2 weeks, spread out | 1-52 | `[owner: amount]` | 7 |
| Red-team author, porter S2, C1 author, two friction raters, desk-audit contractor, two reserve authors | 8 | 1-5 weeks each | various | `[owner: amount]` each | 8 |

The paid time is 18's: about 48 contractor-weeks in all, plus the reserve
authors, which 20 adds (section 3.2 of 20).

## 2. Rules for every role

- **Eligibility.** 18 or older; able to contract with `[owner: contracting
  entity]`; able to write diaries and reports in English; available for
  the role's weeks.
- **Exclusions** (20, section 3.4). Nobody may take part who has committed
  to the Sisu repository, opened an issue or pull request on it, or joined
  its design discussions; who is, or was in the last two years, employed
  by, contracted to or a close collaborator of the owner, the PI or an
  engineer;
  who has argued publicly that ownership without named lifetimes (Hylo,
  Mojo, Vale, Austral or similar) is sufficient or preferable, or has
  contributed to such a language's design (this does not apply to the
  referee); who has written Sisu code other than the proposal's examples
  before onboarding (corpus authors, the red team, porters, the C1 author
  and raters); or who would hold a second role, except that the desk-audit
  contractor may also be the red-team author.
- **Reading the proposal** is allowed and is recorded at screening.
- **Pay does not depend on results.** Fees are fixed per role. An
  implementation delivered as blocked, with the blocking construct named,
  is paid as delivered. Fees are paid `[owner: payment schedule and
  currency]`; the default is one instalment per delivered implementation,
  diary set and questionnaire [default proposed; owner to confirm].
- **Authorship.** Every participant may choose to be named as an author of
  the final report, to be acknowledged, or to stay anonymous. The referee's
  review is published separately under the referee's name (20, section
  11).
- **One channel.** Questions about the work go only to the help desk,
  whose transcript is published. There is no private channel to the team.
- **Blinding.** Participants who write code are asked not to read sections
  2 and 6-8 of 20, or 18's measurement tables, until their last
  deliverable (20, section 3.6).
- **Environment.** Authors, porters and the C1 author work in one provided
  container image with the toolchains, the loggers and the test suites;
  any editor may be used on their own machine.

## 3. Corpus author, non-Rust arm

**What the role does.** Writes two small programs from written
specifications, each in Sisu and in the author's main language (Go, Kotlin
or Swift), in the order the assignment draw gives. Sisu runs on an
interpreter. An implementation is done when the program's shared test
suite passes, or when the author stops and names what blocked them. The
toolchain logs every compiler message and every session; once a week the
author answers a short diary about that week's messages; at the end comes
an exit questionnaire. The programs are drawn from P1-P6 of 20, section
4.1 (a borrowing parser, a graph with parent pointers, an HTTP server with
a shared cache, a widget tree with callbacks, an iterator library, and an
application-shaped program the non-Rust authors choose from three).

**Time and weeks.** About four weeks of paid work (about 20 working days)
between weeks 14 and 32. Onboarding takes about four hours in weeks 14-15.
Program windows: P1, P2, P5 and P6 in weeks 16-23; P4 in weeks 22-27; P3
in weeks 26-30. Each implementation is capped at 60 active hours. The diary
takes about 30 minutes a week and the exit questionnaire about 30 minutes
[estimates: default proposed; owner to confirm].

**Required experience.** Two or more years of production work in Go,
Kotlin or Swift as a main language. Professional experience in one of three
bands, 3-5, 6-10 or 11 or more years; the arm has one author per band, to
match the Rust arm. No Rust beyond tutorials or exercises, about 500 lines
in total at most. One place needs production Go services (P3); one needs
shipped Swift or Kotlin applications with a user interface (P4).

**Excluded experience.** Rust beyond the limit above. Everything in
section 2.

**What the author receives.** A fixed fee of `[owner: amount]`. Authorship
of the report, acknowledgement or anonymity. A help desk, checker bugs
fixed within 24 hours, the shared test suites, and the report.

**Screening.** Sections 9 and 13: an application form, an eligibility
check, a 30-minute interview with one code-reading question in the
author's main language, and selection to fill the bands and the P3 and P4
places.

**Consent and data.** Section 10. In short: logs, session times, diaries,
help-desk messages, answers and code are published under a pseudonym after
the author reviews their diaries; contact and payment details are private
and deleted.

**Schedule.** Interviews in weeks 3-7, contract by week 8 (18), onboarding
in weeks 14-15, programs as above, exit questionnaire within a week of the
last implementation, diary review before publication at about week 50.

## 4. Rust control-group member (corpus author, Rust arm)

**What the role does.** The same as section 3, with Rust as the second
language: two programs, each in Sisu and in Rust, one Sisu-first and one
Rust-first as the draw decides. These implementations are the within-author
control for the bet-1 measures: the Rust versions set the baseline against
which the same author's Sisu friction, time and escape-hatch counts are
compared. Rust-arm authors also answer the exit item on the borrow checker
for Rust as well as for Sisu. Rust may be written with the author's usual
editor tooling; its use is recorded.

**Time and weeks.** As section 3.

**Required experience.** Two or more years of production Rust as a main
language (code shipped to users or deployed). Professional experience in
one of the three bands of section 3, one author per band. One place (P5)
needs the maintainer of a published Rust library that others use.

**Excluded experience.** Section 2, in particular public advocacy of
ownership without named lifetimes and any contribution to Hylo, Mojo's
ownership model, Vale, Austral or a similar design.

**What the author receives.** As section 3, at `[owner: amount]`.

**Screening.** As section 3; the code-reading question is a Rust borrow
error.

**Consent and data.** As section 3.

**Schedule.** As section 3.

## 5. Go port control (porter S1)

**What the role does.** Ports one pre-registered Go service of 3-8 kLOC,
`[owner: service name and repository]`, to Sisu, using the service's own
integration tests translated to Sisu. Then writes ten pre-specified tests
against Sisu's capability fakes (`Fs`, `Clock`, `Net`), and then the same
ten against the Go original with its existing mocking approach; the second
half is the Go control for the test-writing comparison. Keeps the session
logs and a weekly diary that splits time into capability plumbing, other
porting, tests and toolchain problems. A second porter with a Rust
background (porter S2, section 8) ports the same service independently.

**Time and weeks.** Five weeks of paid work between weeks 29 and 40:
onboarding in week 29, the port in weeks 30-38, the tests in weeks 38-39,
the exit questionnaire in week 40. If the bet-2 branch re-ports the worst
module (20, section 8.2), an optional two-week extension in weeks 41-44 at
the same rate [default proposed; owner to confirm].

**Required experience.** Two or more years of production Go services,
including tests that replace I/O with interfaces, mocks or fakes. Rust
experience is recorded, not required or excluded.

**Excluded experience.** Any contribution to the chosen Go service.
Everything in section 2.

**What the porter receives.** A fixed fee of `[owner: amount]`, and the
extension pro rata if used. Authorship, acknowledgement or anonymity. The
help desk and 24-hour bug fixes. The contract has a pause clause: if the
bet-1 interim at week 24 finds a hard fail on P2, 18 pauses the bet-2
spend, and if the port never resumes, a cancellation fee of `[owner:
amount]` is paid [default proposed; owner to confirm].

**Screening.** Sections 9 and 13, with a code-reading question on a Go
test that uses a mock.

**Consent and data.** Section 10.

**Schedule.** Contract by week 12, with the pause clause [default
proposed; owner to confirm]; then as above.

## 6. Go/Java runtime reviewer

**What the role does.** Makes sure the controls of the Phase 1 stack
harness are not mis-tuned, because mis-tuned Go or JVM controls would
flatter Sisu (18's risk table). In week 1 fixes the exact versions and
flags of the controls (Go with `GOMAXPROCS` matched, JDK 21 or later with
virtual threads, tokio, and a C or Rust stackful coroutine library) for the
early-frozen protocol (20, section 1.3). Configures and reviews each
control in the harness. Reproduces every configuration on a second machine
(M2) and writes a short review of the controls' fairness, published
unedited.

**Time and weeks.** Two weeks of paid work between weeks 1 and 10: week 1
for versions and flags, weeks 2-6 for configuration and review, weeks 6-8
for the reproduction on M2, and weeks 9-10 only if a decision branch reruns
the harness.

**Required experience.** Production experience tuning Go and JVM runtimes
(garbage collectors, schedulers, virtual threads), and measuring
performance on Linux (resident-set accounting, cgroups, CPU pinning).
Access to a Linux x86-64 machine that meets 20's section 4.4 (at least 16
GiB of RAM, a kernel 6.1 or later with io_uring, a CPU vendor or generation
different from M1's), or use of one provided by the project: `[owner:
machine for M2]`.

**Excluded experience.** Section 2.

**What the reviewer receives.** A fixed fee of `[owner: amount]`; the
review published under their name; authorship, acknowledgement or
anonymity.

**Screening.** Sections 9 and 13, with the runtime questions of the script
in place of a code-reading question.

**Consent and data.** Section 10; the reviewer's machine specification and
review are published.

**Schedule.** Contracted before week 1, because the work starts in week 1
[default proposed; owner to confirm].

## 7. Independent referee

**What the role does.** A publicly sceptical Rust expert whose job is to
make the study hard to fool. The referee:

1. reviews and co-signs the pre-registration in weeks 2-4, and co-signs or
   refuses every later amendment;
2. classifies a 10% sample of the lifetime desk audit without seeing the
   contractor's labels;
3. writes 20 adversarial programs that try to alias or dangle in safe Sisu
   code (weeks 9-14); outsiders start only if the checker rejects all of
   them;
4. supplies the random seed for the assignment draw, and checks the
   eligibility record before contracts are signed;
5. runs every threshold comparison from the raw logs at weeks 8, 24, 32,
   40, 46 and 50, and commits the outputs, which are the numbers of record;
6. audits the triage labels that exclude checker bugs from friction,
   classifies god-struct candidates, and checks the seeded-bug list
   against its selection rule;
7. writes a review published unedited with the report, including
   disagreements (weeks 48-51).

**Time and weeks.** About two weeks of paid work (ten to twelve days)
spread over weeks 1-52: about one day for the pre-registration, half a day
for the audit sample, four days for the adversarial programs, half a day at
each of six decision points, half a day for the audits, and three days for
the review [default proposed; owner to confirm].

**Required experience.** Rust expertise at the level of a recognised
library or compiler contributor, and a public record (talks, posts, papers
or reviews) of scepticism that ownership without named lifetimes is
expressive enough.

**Excluded experience.** Any contribution to the Sisu repository or its
design; a working relationship with the team in the last two years; any
other role in the study. The advocacy exclusion of section 2 does not
apply; the referee is chosen for the opposite view.

**What the referee receives.** A fixed fee of `[owner: amount]`, the same
whatever the conclusions. The review is published unedited under the
referee's name. Independence terms [default proposed; owner to confirm]:
the contract ends only for non-delivery, never for disagreement; the
referee may publish their own commentary anywhere at any time after the
verdict it concerns; the referee may decline authorship of the report; if
the referee withdraws, the owner names a replacement who meets the same
criteria, and the change is recorded as a deviation.

**Screening.** Sections 9 and 13, with the referee's questions of the
script.

**Consent and data.** Section 10; the referee is named, not pseudonymous.

**Schedule.** Contracted before week 1, because the pre-registration must
carry the referee's sign-off by week 4 [default proposed; owner to
confirm].

## 8. Other paid roles

These roles follow sections 2, 9 and 10 like the others.

| Role | Places | What the role does | Paid time and weeks | Required | Excluded beyond section 2 | Fee |
|---|---|---|---|---|---|---|
| Red-team author | 1 | implements the ten patterns for which Rust needs lifetimes (20, section 4.1.1) in Sisu and in Rust, and reports per pattern what Sisu cannot express and what each workaround costs | 3 weeks, weeks 20-25 | five or more years of Rust; has written code using the patterns or close relatives | none; may also be the desk-audit contractor | `[owner: amount]` |
| Porter S2 | 1 | the second, independent port of the Go service; the ten tests with Sisu fakes | 5 weeks, weeks 29-40, with the extension and pause clause of section 5 | two or more years of production Rust | contributors to the chosen Go service | `[owner: amount]` |
| C1 author | 1 | grows `wordcount.sisu` into a grep/du-style command-line tool with arguments, `Fs`, `Env` and `Proc` | 3 weeks, weeks 28-34 | a student, or under two years of professional programming | none | `[owner: amount]` |
| Friction raters | 2 | code every diary and log event under 20's codebook, independently, then reconcile | 1 week each: calibration in week 23, weeks 24-26, weeks 31-32, one day in week 41 | reads Rust and at least one of Go, Kotlin or Swift fluently; code review or qualitative coding | none | `[owner: amount]` |
| Desk-audit contractor | 1 | classifies every explicit lifetime in about ten Rust crates (20, section 4.6) | 2 weeks, weeks 1-4 | five or more years of Rust | none | `[owner: amount]` |
| Reserve authors | 2, one per arm | on standby to replace a dropout, or to write the re-runs of 20's section 8.1 | a retainer for weeks 8-40; the corpus-author rate when activated, up to about four weeks | as the arm covered (sections 3 and 4) | none | retainer `[owner: amount]`; fee `[owner: amount]` |

The reserve authors are 20's addition to 18 [default proposed; owner to
confirm]. The desk-audit contractor is contracted before week 1, because
the audit runs in weeks 1-4.

## 9. Screening procedure

1. **Application form.** Role; main languages and years in each;
   professional experience band; for the non-Rust arm, Rust experience in
   lines; optional links to public code or a CV; availability for the
   role's weeks; the conflict declarations of section 2; which parts of the
   Sisu proposal the applicant has read. Nothing else is asked: no age,
   gender, location or employer [default proposed; owner to confirm].
2. **Eligibility check** by the PI against sections 2-8. The reason for
   each rejection is recorded under a pseudonym in the eligibility record.
3. **Interview**: 30 minutes by video or voice, by the PI, with the script
   in section 13. Notes record only the answers that decide eligibility.
   The call is not recorded [default proposed; owner to confirm].
4. **Selection** fills the places: for corpus authors, one per experience
   band in each arm, plus the P3, P4 and P5 places. Where several eligible
   applicants fit one place, the PI picks to widen the spread of domains
   and employers across the arm, and a recorded random draw breaks any
   remaining tie [default proposed; owner to confirm].
5. **Referee check.** The referee checks the eligibility record before any
   contract is signed (20, section 3.4).
6. **Contract and consent** forms are signed.
7. **Assignment.** After the last corpus-author contract, the assignment
   draw gives each author two programs and an order (20, section 3.5),
   from a script committed before freeze and a seed the referee supplies.
8. **Applicants not selected** are told within two weeks of their
   interview. Their application data are deleted after selection unless
   they agree to be kept as a reserve [default proposed; owner to confirm].

## 10. Consent and data terms

The consent form says, in plain words:

1. **What is recorded.** Every compiler message and test run, the times of
   saves and checks within work sessions, weekly diaries, help-desk
   messages, questionnaire answers and every version of the code written
   for the study.
2. **What is published.** All of the above, with the report, under a
   pseudonym (for example R2 or N1). Session times are stored as offsets
   from the participant's first session, not as clock times. Diaries are
   published only after their author has reviewed them; personal content
   may be redacted, technical content may not, and each redaction is
   marked.
3. **What stays private.** Name, contact and payment details. They are
   kept by the PI apart from the data, never committed to the repository,
   and deleted `[owner: retention period]` after the report.
4. **Identifiability.** With six corpus authors whose backgrounds are
   described, people who know a participant may recognise them. A
   participant who accepts authorship is named as an author, but not
   linked to a pseudonym.
5. **Withdrawal.** A participant may withdraw at any time without giving a
   reason and is paid for the work done. They choose whether the data
   collected so far are kept and published or deleted. The report counts
   withdrawals either way.
6. **Licence.** Code and diaries are published under `[owner: licence]`.
   Programs may be reused inside the study (for example, copies of P3 carry
   the seeded bugs of 20, section 4.9).
7. **Blinding request.** Not to read sections 2 and 6-8 of 20, or 18's
   measurement tables, until the participant's last deliverable.
8. **Pay.** Fixed, and the same whatever the programs or diaries show.
9. **Responsibility.** The data controller is `[owner: data controller]`
   under `[owner: applicable data-protection law]`; the study's ethics
   review is `[owner: review body and approval, or the reason none is
   required]`; questions about data go to `[owner: contact address]`.

## 11. Recruitment schedule

| Week | Step |
|---|---|
| before 1 | the reviewer, the referee and the desk-audit contractor are approached directly and contracted, because their work starts in weeks 1-2 [default proposed; owner to confirm] |
| 1 | the call (section 12) is posted at `[owner: where the call is posted]`, where Rust, Go and Swift or Kotlin developers read (18) |
| 4 | the pre-registration is frozen; its hash is added to the posted call |
| `[owner: closing date]` | applications close; default end of week 5 |
| 3-7 | interviews |
| 8 | contracts signed with the six corpus authors, the red-team author and the two reserve authors (18: by week 8); assignment draw |
| 12 | contracts signed with the porters (with the pause clause), the C1 author and the raters [default proposed; owner to confirm] |
| 14-15 | corpus-author onboarding |
| 16 | P1, P2, P5 and P6 start |
| 20 | the red team starts |
| 22 | P4 starts |
| 23 | the raters calibrate |
| 24 | the bet-1 interim; a hard fail on P2 pauses the porters' contracts |
| 26 | P3 starts |
| 28 | C1 starts |
| 29 | porter onboarding |
| 30 | the ports start |
| 32 | the bet-1 verdict |
| 40 | the bet-2 verdict; the last participant deliverables |
| 41-44 | the porters' optional extension, if the bet-2 branch needs it |
| about 50 | participants review their diaries before publication |
| 52 | the report is published |

## 12. Call for participants

The text below is posted as it stands once the placeholders are filled.

---

**Paid programmers wanted to test a new language design**

The Sisu project is a proposal for a statically typed programming language
with indentation-based syntax, ownership without a garbage collector, and
capabilities for I/O. Before building a compiler, the project is testing
the three parts of the design most likely to be wrong, on an interpreter.
The programs that decide the tests must be written by people who did not
design the language, so the project pays outsiders to write them. Week 1
of the study begins on `[owner: date of week 1]`.

**Open places**

| Role | Places | Work | When | Fee |
|---|---|---|---|---|
| Corpus author: Go, Kotlin or Swift developer with no Rust | 3 | two programs of 300-1500 lines, each in Sisu and in your main language | about 4 weeks of work in weeks 14-32 | `[owner: amount]` |
| Corpus author: Rust developer | 3 | two programs, each in Sisu and in Rust | about 4 weeks of work in weeks 14-32 | `[owner: amount]` |
| Red-team Rust expert | 1 | the patterns that need lifetimes in Rust, attempted in Sisu | 3 weeks, weeks 20-25 | `[owner: amount]` |
| Porter, Go background | 1 | port a 3-8 kLOC Go service to Sisu; write ten tests in Sisu and the same ten in Go | 5 weeks, weeks 29-40 | `[owner: amount]` |
| Porter, Rust background | 1 | the same port, independently; ten tests in Sisu | 5 weeks, weeks 29-40 | `[owner: amount]` |
| Command-line tool author: student or junior developer | 1 | grow a small grep/du-style tool in Sisu | 3 weeks, weeks 28-34 | `[owner: amount]` |
| Friction rater | 2 | code programmers' diaries and compiler logs | 1 week, in weeks 23-41 | `[owner: amount]` |
| Reserve author | 2 | stand by to replace an author who leaves | a retainer, weeks 8-40 | `[owner: amount]` |
| Go/Java runtime reviewer | 1 | configure the Go and JVM benchmark controls and reproduce the benchmarks | 2 weeks, weeks 1-10 | `[owner: amount]` |
| Lifetime desk auditor: Rust expert | 1 | classify every explicit lifetime in about ten Rust crates | 2 weeks, weeks 1-4 | `[owner: amount]` |
| Referee: a Rust expert publicly sceptical of ownership without named lifetimes | 1 | check the study's rules, try to break the checker, rerun every analysis, publish an unedited review | about 2 weeks over the year | `[owner: amount]` |

The reviewer, desk-auditor and referee places are filled first and may already be taken.

**Who can apply.** Anyone 18 or older who can contract with `[owner:
contracting entity]`, write in English, and meet the role's requirements in
the recruitment brief (`proposal/21-recruitment.md` in `[owner: repository
address]`). Corpus authors are chosen to cover three bands of professional
experience (3-5, 6-10 and 11 or more years) in each group.

**Who cannot apply.** Contributors to the Sisu repository or its design
discussions; anyone who has worked for or closely with the Sisu team in the
last two years; anyone who has argued publicly that ownership without named
lifetimes is sufficient or better, or helped design such a language
(except for the referee place); anyone who has already written Sisu code.

**What you get.** A fixed fee, paid whatever your programs or diaries
show; a program you could not finish, with the reason stated, is paid as
delivered. Your choice of authorship of the final report, acknowledgement
or anonymity. A help desk, checker bugs fixed within 24 hours, and the
test suites for your programs.

**What is recorded and published.** Compiler messages, work-session times
(as offsets, not clock times), weekly diaries, help-desk messages,
questionnaire answers and your code, under a pseudonym, after you have
reviewed your diaries. Name, contact and payment details stay private and
are deleted after the report. With few participants, people who know you
may recognise you. You can withdraw at any time and are paid for work
done.

**The rules are fixed in advance.** The measurements, thresholds and
decisions are pre-registered in `proposal/20-pre-registration.md`, whose
frozen version has the hash `[to be filled at freeze]`. If you are
selected, you will be asked not to read its measurement sections until you
have finished.

**How to apply.** `[owner: application form or address]` by `[owner:
closing date]`. Eligible applicants are invited to a 30-minute interview.
Questions: `[owner: contact address]`.

---

## 13. Screening interview script

**Rules for the interviewer.** Use the same script for every applicant to a
role. Do not say what results the team expects, and do not name any
threshold. Record only the answers to questions 1-8, in the eligibility
record, under a pseudonym. If the applicant is not eligible, say so at the
end with the criterion that applies.

**Opening (read aloud).** "Thank you for applying. This call takes about 30
minutes. I will ask about your experience and possible conflicts of
interest, and explain what the study records. I note only the answers that
decide eligibility, and the call is not recorded. I will not describe what
results we expect. You can stop at any time."

1. **Role and time.** "Which role are you applying for? Are you available
   for its weeks, from `[date]` to `[date]`?"
2. **Experience.** "Which languages have you used as your main language at
   work, and for how many years each? How many years have you programmed
   professionally in total?" (Record the band.)
3. **Role-specific questions.**
   - Non-Rust arm: "Have you written any Rust? Roughly how much, and
     when?" (Eligible only with tutorials or exercises, about 500 lines at
     most.) "Describe the last Go, Kotlin or Swift system you shipped or
     deployed, and your part in it." For the P3 place: "Have you built and
     run Go backend services in production?" For the P4 place: "Have you
     shipped a Swift or Kotlin application with a user interface?"
   - Rust arm: "Describe the largest Rust codebase you shipped or
     deployed, and your part in it. How did it handle shared mutable
     state?" For the P5 place: "Which published Rust library do you
     maintain, and who uses it?"
   - Porters: "Describe a service you worked on and how its tests replaced
     I/O: interfaces, mocks or fakes." (S1: Go; S2: Rust.) "Have you
     contributed to `[service]`?"
   - Red team and desk audit: "Which of these have you written in Rust:
     self-referential structs, structs with more than one lifetime, stored
     references, intrusive lists, cursor APIs?"
   - C1 author: "Are you a student, or how long have you programmed
     professionally?"
   - Raters: "Which of Rust, Go, Kotlin and Swift do you read fluently?
     Describe any code review or qualitative coding you have done."
   - Reviewer: "Which Go or JVM runtime settings have you tuned in
     production, and how did you measure the effect? Can you use a Linux
     x86-64 machine with at least 16 GiB of RAM for two weeks?"
   - Referee: "Where have you written or spoken about ownership without
     named lifetimes, and what did you argue? What evidence would change
     your view?" Then read the independence terms of section 7 and ask
     whether they are acceptable.
4. **Code reading** (authors, porters, red team; about five minutes). Show
   one short program with one compiler error in the applicant's main
   language: a borrow error for Rust, a mock-based Go test that fails for
   porter S1, an error from the applicant's own language otherwise. Ask
   what the error means and how they would fix it. This checks the
   experience claimed; it is not scored beyond that.
5. **Conflicts.** "Have you committed to the Sisu repository, opened an
   issue or pull request on it, or joined its design discussions?" "In the
   last two years, have you worked for or closely with `[owner]`, the PI or
   the Sisu engineers?" "Have you argued publicly that ownership without
   named lifetimes, as in Hylo, Mojo, Vale or Austral, is sufficient or
   better, or helped design such a language?" (Not asked of the referee.)
6. **Prior exposure.** "Have you read any of the Sisu proposal? Which
   parts? Have you written any Sisu code?"
7. **Data and consent (read aloud).** Summarise section 10: what is
   recorded and published, pseudonyms and the identifiability caveat, the
   diary review, withdrawal, and pay that does not depend on results. "Is
   that acceptable to you?"
8. **Blinding.** "If you are selected, we will ask you not to read sections
   2 and 6-8 of the pre-registration, or the measurement tables in
   document 18, until your last deliverable. Can you agree to that?"
   (Asked of authors, porters, the red team and the C1 author.)
9. **The applicant's questions.** Answer factually. Do not describe
   expected results or thresholds.

**Closing (read aloud).** "We will tell you our decision by `[date]`. If
you are not selected, your application data are deleted after selection,
unless you agree to be kept as a reserve."

## 14. Open items for the owner

Placeholders in this brief, one line each:

1. Every fee: each role in sections 1 and 8, the reserve retainer, and the
   porters' cancellation fee and extension rate.
2. The payment schedule and currency (section 2).
3. The contracting entity (section 2).
4. The date of week 1 (introduction and section 12).
5. Where the call is posted (section 11).
6. The application form or address, the closing date and the contact
   address (sections 11 and 12).
7. The repository address for the call (section 12).
8. The Go service's name and repository, for the porters' role text and
   conflict question (sections 5 and 13).
9. A machine for M2 if the reviewer has none (section 6).
10. The retention period, the licence, the data controller and the
    data-protection law, and the ethics review (section 10; shared with 20,
    section 13).
11. The pre-registration hash, added to the posted call at freeze
    (section 12).
