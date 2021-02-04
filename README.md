Discord bot based on [serenity](https://github.com/serenity-rs/serenity)




## Plan

* Per server:
    * Define a channel as a "story channel" with [config]
    * Calculate stats for that server
    

### story-channel config
* Whether to look historically (Might be necessary if adding to huge existing stories)
* Whether to keep track of timing and who's up next and whether to alert


### Word Frequencies:
Writing down thoughts on storing word frequencies:
* Dictionaries are big, a hashmap of every word in the dictionary -> usize would be ... big

* Updating a given StoryStats dictionary:
  * Messages are added to an "unprocessed" list
  * The StoryStats has some Summary type dictionary of say top N words, not full dictionary
  * Out of band system finds StoryStatses with unempty unprocessed lists and:
    * loads specific dictionary from disk
    * updates that dictionary with the unprocessed messages
    * clears the unprocessed messages
    * Smashes the new stats summary into the struct
    * saves the dictionary back to disk