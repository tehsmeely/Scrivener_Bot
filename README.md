# Scrivener
#### *Discord Bot*

## What Does it Do?
If you have channels where 1+ Users are writing stories in sequential messages,
this bot can be used on the server hosting such stories to report on stats about these stories

## What Stats?

Right now, the bot will give you the following stats:
* Word Counts: For the everyone in the whole channel, or per user
* Word Frequencies: Reports the top most used words, again for everyone or a specific user
* Word Clouds: Generates and attached a word cloud image of the words used by everyone or a specific user in the channel 


## Small TODOS:
* Wordcloud response should include the context: "Here's a wordcloud for User in Channel"
* Arg errors should probably be more clear and just re-iterate what they were expecting

## Expansion Features:
* Server-wide info
  * Cross-channel summaries (Top most posted channels, summaries across them for the user)
  * Server-wide wordclouds per user or general
  * Required for the above: Display and management of initialised channels 
---

## Technical Stuff
Built using Rust and [serenity](https://github.com/serenity-rs/serenity)

Suggested bot permissions: 523344 (Specifically Read Messages + All Text channel options)
To add to a server, click [here](https://discord.com/api/oauth2/authorize?client_id=805918656622100500&permissions=523344&scope=bot)


### TODO:
* Admin/Role control for initialising channels
  
* Make arg handling clearer
  * Cleaner code
  * more obvious errors on invalid args
  
* Report errors in commands better:
  * Reduce the number of lazy [.unwrap()] usages
  
* Clean up the wordcloud code a bit
  * Use centralised config for the file paths
  * Generally sanitise code, wrap stuff in functions maybe, handle errors

### On how to handle lots of Word Frequencies and not run out of memory
Writing down thoughts on storing word frequencies:
* Dictionaries are big, a hashmap of every word in the dictionary -> usize would be ... big if we are making one per user per channel

* Updating a given StoryStats dictionary:
  * Messages are added to an "unprocessed" list
  * The StoryStats has some Summary type dictionary of say top N words, not full dictionary
  * Out of band system finds StoryStatses with unempty unprocessed lists and:
    * loads specific dictionary from disk
    * updates that dictionary with the unprocessed messages
    * clears the unprocessed messages
    * Smashes the new stats summary into the struct
    * saves the dictionary back to disk
  
### On the WordCloud system
The principle of the system is:
* There's a python subprocess running for the lifetime of the process, spawned by the startup of the binary
* The process is:
  * watching for json files in some "in" dir
  * parses the files as word frequency dicts (string -> float)
  * generates a wordcloud image and saves it to some "out" dir
* Inside the bot binary when we want to create a wordcloud:
  * Serialise the word frequency hashmap and save to a json file in the "in" dir
  * Wait patiently for the result image to appear in the "out" dir
  * Gives up if it doesnt appear after some timeout
  * If it appears, attach it to an image and send
* Files for in and out are named using a uuid, the bot generates them per request and the python process keeps track of those it has handled
