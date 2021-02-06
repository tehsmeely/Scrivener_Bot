Discord bot based on [serenity](https://github.com/serenity-rs/serenity)



Suggested bot permissions: permissions=523344

https://discord.com/api/oauth2/authorize?client_id=805918656622100500&permissions=523344&scope=bot


## TODO:
* 



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