# Scheduling

A schedule launches profiles automatically.

## Target

A schedule applies to one of:

- a single profile
- every profile with a tag
- every profile in a group

## Recurrence

- **Daily** at a time `HH:MM` in the schedule's timezone
- **Weekdays** at a time on chosen days (Monday to Sunday)
- **Every N hours**, counted from the schedule's anchor time

## Action

- **Warm**: open each URL in a list, stay a set number of seconds on each with light scrolling, then close.
- **Refresh browser**: check the Camoufox version. It does not change the pinned hardware fingerprint.

## Preconditions

Before each run the scheduler checks:

- the profile is closed (default: wait up to 60 seconds for it to close, otherwise skip)
- the assigned proxy passes a fresh check (default: on)

If a check fails, that run ends and the failure is recorded. A schedule that is already running is not
started again.
