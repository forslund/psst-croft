# Psst-croft

This is an experiment for using [psst](https://github.com/jpochyla/psst) as a spotify player for [Mycroft A.I.](https://mycroft.ai).

Current version is based on the psst-cli and psst-gui code.

Search of albums and start of playback (of albums) through mycroft messagebus is
working:

The `spotify.search` message searches for an album given in the `query` field of the message data and returns the response in a `spotify.search.response` message.

The `spotify.play` message will try to play the id given in the `album` field of the message data as an album.


## TODO:

- [x] Connection to Mycroft messagebus
 - [x] Search message handler
  - Handle different kinds of searches (currently just albums)
 - [x] Play message handler
  - Handle different kinds of types (currently just albums)
- Code cleanup
- Remove druid dependency
- [x] Figure out Why "stop" isn't executed, Psst upstream issue
- Handle the psst-core dependency in a better way (currently requires that the psst repo is checked out next to psst-croft
