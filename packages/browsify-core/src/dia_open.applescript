on run argv
    set requestedName to item 1 of argv
    set targetURL to item 2 of argv
    set shouldFocus to item 3 of argv is "true"
    if not (application id "company.thebrowser.dia" is running) then return "not-running"
    tell application id "company.thebrowser.dia"
        with timeout of 10 seconds
            if (count windows) is 0 then return "no-window"
            set destinationWindow to front window
            -- Dia's remote `whose name is` filter ignores AppleScript's case setting.
            -- Compare returned names locally before creating anything.
            set matches to {}
            repeat with candidate in profiles of destinationWindow
                set candidateName to name of candidate
                considering case, diacriticals, hyphens, punctuation, white space
                    if candidateName is requestedName then set end of matches to contents of candidate
                end considering
            end repeat
            if (count matches) is 0 then return "missing-profile"
            if (count matches) is not 1 then return "ambiguous-profile"
            set destinationProfile to item 1 of matches
            set newTab to make new tab at end of tabs of destinationProfile with properties {URL:targetURL}
            if shouldFocus then focus newTab
            return "opened"
        end timeout
    end tell
end run
