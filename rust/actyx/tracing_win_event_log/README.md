# tracing_win_event_log

## Info

Winlog and tracing-journld crates were used for inspiration (read copy/paste/edit).
`eventmsgs.lib` is generated from `eventmsgs.mc` via `mc` and gets embeded via `build.rs` into the executable.
An entry needs to be created in windows registry under `HKLM\System\CurrentControlSet\Services\EventLog\Application\ActyxOS` registry key. 
This entry needs to contain string value with name `EventMessageFile` and value full path to the executable (like `C:\Users\moshensky\source\repos\cosmos\rt-master\target\debug\tracing_win_event_log_sample.exe`).
Above registry key is used by Event Log to read metadata about log messages.


## Various powershell commands 

Createa event log (can be used when installing app):

```psh
new-eventlog -source ActyxFoo -logname Application -MessageResourceFile "C:\Users\moshensky\source\repos\cosmos\rt-master\target\debug\tracing_win_event_log_sample.exe" 
```

Remove event source (can be used when uninstalling app):

```psh
remove-eventlog -source ActyxFoo
```

Create event source:
```psh
[System.Diagnostics.EventLog]::CreateEventSource("ActyxFoo", "Application")
```

Delete event source:

```psh
[System.Diagnostics.EventLog]::DeleteEventSource("ActyxFoo")
```

Get last event:

```psh
Get-EventLog -Newest 1 -LogName Application -Source ActyxOS2 | Select-Object Source, EntryType, EventID, Message | foreach { "$_" }
```


Write log entry:

```psh
[System.Diagnostics.EventLog]::WriteEntry("ActyxFoo", "This is a sample event entry", "Information", 100)
```

## Links

Event Log interaction - https://www.codeproject.com/Articles/4166/Using-MC-exe-message-resources-and-the-NT-event-lo