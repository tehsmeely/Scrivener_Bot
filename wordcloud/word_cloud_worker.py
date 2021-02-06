import wordcloud
import os, re, json, time, io

watch_path = "D:\\Library\\Documents\\rust\\StoryStatsWatcher\\wordcloud\\working\\in"
output_dir = "D:\\Library\\Documents\\rust\\StoryStatsWatcher\\wordcloud\\working\\out"

file_regex = re.compile(r'(.*).generate.json')
output_file_name_template = "{}.generated.png"

def make_image_and_save(freq_data, request_id):
    wc = wordcloud.WordCloud(background_color="black", max_words=1000)
    wc.generate_from_frequencies(freq_data)
    output_file_name = output_file_name_template.format(request_id)
    output_path = os.path.join(output_dir, output_file_name)
    wc.to_file(output_path)


def read_freq_data_from_file(filename):
    with io.open(filename, mode="r", encoding="utf-8") as f:
        return json.load(f)

def search_for_new_files(ids_handled, delay = 0.4):
    print("Searching for new files in {}".format(watch_path))
    while True:
        for dir_entry in os.scandir(watch_path):
            if dir_entry.is_file():
                match = file_regex.match(dir_entry.name)
                if match:
                    request_id = match.group(1)
                    if request_id not in ids_handled:
                        ids_handled.append(request_id)
                        return (dir_entry.path, request_id)
        time.sleep(delay)


def main():
    ids_handled = []
    while True:
        (file_to_process, request_id) = search_for_new_files(ids_handled)
        print("Found {} to process".format(file_to_process))
        data = read_freq_data_from_file(file_to_process)
        print("Successfully read file, creating wordcloud")
        make_image_and_save(data, request_id)


if __name__ == "__main__":
    main()
