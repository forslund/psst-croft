# Psst-croft

This is an experiment for using [psst](https://github.com/jpochyla/psst) as a spotify player for [Mycroft A.I.](https://mycroft.ai).

Current version is based on the psst-cli and psst-gui.

Currently no integration is happening, this is just a poc for searching and then playing an album.

## TODO:

- Connection to Mycroft messagebus
 - Search message handler
 - Play message handler
- Code cleanup
- Remove druid dependency
- Figure out Why "stop" isn't executed
- Handle the psst-core dependency in a better way (currently requires that the psst repo is checked out next to psst-croft
