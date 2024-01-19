# Home DevOps

Tool to automate deploy for home projects

## Commands

```bash
hmd init [SERVER_ADDRESS]
```
Creates bare git repository at `server` and common directories.
Add remote to current dir git repository.
Creates `hmd.yml` file with default template.
_Uses dirname as project name._

```bash
hmd deploy
```
Pushs to server and run scripts from `hmd.yml`.
Saves app instant pid if launch script runned.

```bash
hmd info
```
Prints info about last deploy and app instant if running.

```bash
hmd log
```
Prints app instant logs.

```bash
hmd stop
```
Stops running app instant.

```bash
hmd run
```
Runs app instant.
