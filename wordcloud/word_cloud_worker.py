import wordcloud
import numpy
import PIL, PIL.ImageOps
import os, re, json, time, io, sys

IDS_HANDLED_FILENAME = "ids_handled.txt"
MASKS = [("d20.png", "d20"), ("bunny.png", "bunny"), ("shield.png", "shield"), ("wolf.png", "wolf"), ("horse.png", "horse")]
MAX_MASK_DIM = 500

#file_regex = re.compile(r'(.*).generate.json')
output_file_name_template = "{}.generated.png"

file_regex = re.compile(r'([a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}).(?:([A-z0-9]+).)?generate.json')


def load_masks():
    masks = {}
    for mask, mask_name in MASKS:
        img = PIL.Image.open(os.path.join("wordcloud\\masks", mask))
        img = img.convert('L')
        scale = MAX_MASK_DIM / max(img.size[0], img.size[1])
        img = PIL.ImageOps.scale(img, scale)
        sys.stderr.write("{}".format(img.size))
        masks[mask_name] = numpy.array(img)
    return masks


def make_image_and_save(freq_data, request_id, output_dir, mask):
    wc = wordcloud.WordCloud(background_color="black", max_words=1000, mask=mask)
    wc.generate_from_frequencies(freq_data)
    output_file_name = output_file_name_template.format(request_id)
    output_path = os.path.join(output_dir, output_file_name)
    wc.to_file(output_path)


def read_freq_data_from_file(filename):
    with io.open(filename, mode="r", encoding="utf-8") as f:
        return json.load(f)


def search_for_new_files(watch_path, ids_handled, delay=0.4):
    sys.stderr.write("Searching for new files in {}".format(watch_path))
    while True:
        for dir_entry in os.scandir(watch_path):
            if dir_entry.is_file():
                match = file_regex.match(dir_entry.name)
                if match:
                    request_id = match.group(1)
                    if request_id not in ids_handled:
                        ids_handled.append(request_id)
                        mask_name = match.group(2)
                        return (dir_entry.path, request_id, mask_name)
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
    masks = load_masks()
    print("Watching for files at {}\nOutputting files to {}".format(request_path, generated_image_path))
    sys.stdout.write("Watching for files at {}\nOutputting files to {}\n".format(request_path, generated_image_path))
    while True:
        (file_to_process, request_id, mask_name) = search_for_new_files(request_path, ids_handled)
        print("Found {} to process".format(file_to_process))
        data = read_freq_data_from_file(file_to_process)
        print("Successfully read file, creating wordcloud")
        if mask_name is not None:
            mask = masks.get(mask_name)
        else:
            mask = None
        make_image_and_save(data, request_id, generated_image_path, mask)
        dump_handled_ids(request_path, ids_handled)


if __name__ == "__main__":
    main()
