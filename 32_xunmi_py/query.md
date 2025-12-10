# xunmi-py is not neccessary.
# 1. cargo build with src/lib.rs and build.rs
# 2. rename to xunmi.so and run ipython
# 3. [ipython] from xunmi import *
# ...
# maturin: package a rust-based python code to whl(eg: xunmi).

from xunmi import *

indexer = Indexer("./fixtures/config.yml")
updater = indexer.get_updater()
f = open("./fixtures/wiki_00.xml")
data = f.read()
f.close()
input_config = InputConfig("xml", [("$value", "content")], [("id", ("string", "number"))])
updater.update(data, input_config)
updater.commit()

result = indexer.search("历史", ["title", "content"], 5, 0)
result
