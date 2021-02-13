import wordcloud
import os, re, json, time, io, sys

IDS_HANDLED_FILENAME = "ids_handled.txt"

file_regex = re.compile(r'(.*).generate.json')
output_file_name_template = "{}.generated.png"


def make_image_and_save(freq_data, request_id, output_dir):
    wc = wordcloud.WordCloud(background_color="black", max_words=1000)
    wc.generate_from_frequencies(freq_data)
    output_file_name = output_file_name_template.format(request_id)
    output_path = os.path.join(output_dir, output_file_name)
    wc.to_file(output_path)


def read_freq_data_from_file(filename):
    with io.open(filename, mode="r", encoding="utf-8") as f:
        return json.load(f)


def search_for_new_files(watch_path, ids_handled, delay=0.4):
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


def dump_handled_ids(path, ids_handled):
    with open(os.path.join(path, IDS_HANDLED_FILENAME), "w") as f:
        for id in ids_handled:
            f.write("{}\n".format(id))


def load_handled_ids(path):
    fname = os.path.join(path, IDS_HANDLED_FILENAME)
    if os.path.isfile(fname):
        ids = []
        with open(fname, "r") as f:
            for line in f:
                ids.append(line)
        return ids
    else:
        return []


def main():
    if len(sys.argv) != 3:
        raise ValueError("Invalid number of args")
    request_path = sys.argv[1]
    generated_image_path = sys.argv[2]
    ids_handled = load_handled_ids(request_path)
    print("Watching for files at {}\nOutputting files to {}".format(request_path, generated_image_path))
    while True:
        (file_to_process, request_id) = search_for_new_files(request_path, ids_handled)
        print("Found {} to process".format(file_to_process))
        data = read_freq_data_from_file(file_to_process)
        print("Successfully read file, creating wordcloud")
        make_image_and_save(data, request_id, generated_image_path)
        dump_handled_ids(request_path, ids_handled)


if __name__ == "__main__":
    main()
