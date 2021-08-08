# Debugging built MSI

If the installation fails for some reason you can debug what happened using the following command and checking `log.txt`:

```powershell
msiexec.exe /i actyx-2.0.0-windows-x64.msi /Liwearucmov*x log.txt
```
