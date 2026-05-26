# Refreshing Streams

This guide explains manual refreshes and automatic polling.

## Manual Refresh

Use **Refresh** to manually refresh the selected saved query. When a library
view is selected, manual refresh runs all enabled saved queries.

Refresh results are written to the local database in the background. If the
results would change the item list that you are currently reading, the list
stays in place and an update banner appears. Select **Show updates** when you
are ready to display the refreshed list.

If authentication, network, or API errors happen during refresh, previously
stored items remain visible.

## Automatic Polling

The app also polls automatically. The default polling interval is 180 seconds.
Use **Preferences** > **Polling interval** to change the interval and save it to
the YAML configuration file. As with manual refresh, remote changes that affect
the visible item list appear behind the **Show updates** banner rather than
reordering the list while you are reading it.
