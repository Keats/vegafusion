# VegaFusion
Serverside acceleration for the Vega visualization grammar

## Quick Start: Serverside acceleration for Altair in Jupyter
VegaFusion can be used to provide serverside acceleration for Altair visualizations when displayed in Jupyter contexts (Classic notebook, JupyterLab, and Voila). First, install the `vegafusion-jupyter` package, along with `vega-datasets` for the example below.

```bash
pip install vegafusion-jupyter vega-datasets
```

Then open a jupyter notebook (either the classic notebook, or a notebook inside JupyterLab), and run these two lines to import and enable VegaFusion

```python
import vegafusion_jupyter as vf
vf.enable()
```
VegaFusion will now be used to accelerate any Altair chart. For example, here's the [interactive average](https://altair-viz.github.io/gallery/selection_layer_bar_month.html) Altair gallery example.

```python
import altair as alt
from vega_datasets import data

source = data.seattle_weather()
brush = alt.selection(type='interval', encodings=['x'])

bars = alt.Chart().mark_bar().encode(
    x='month(date):O',
    y='mean(precipitation):Q',
    opacity=alt.condition(brush, alt.OpacityValue(1), alt.OpacityValue(0.7)),
).add_selection(
    brush
)

line = alt.Chart().mark_rule(color='firebrick').encode(
    y='mean(precipitation):Q',
    size=alt.SizeValue(3)
).transform_filter(
    brush
)

chart = alt.layer(bars, line, data=source)
chart
```

https://user-images.githubusercontent.com/15064365/148408648-43a5cfd0-b0d8-456e-a77a-dd344d8d07df.mov

Histogram binning, aggregation, selection filtering, and average calculations will now be evaluated in the Python kernel process with efficient parallelization, rather than in the single-threaded browser context.

You can see that VegaFusion acceleration is working by noticing that the Python [kernel is running](https://experienceleague.adobe.com/docs/experience-platform/data-science-workspace/jupyterlab/overview.html?lang=en#kernel-sessions) as the selection region is created or moved. You can also notice the VegaFusion logo in the dropdown menu button.

## Background: Vega, Vega-Lite, and Altair
VegaFusion is designed to complement the Vega visualization ecosystem. In particular, the [Vega](https://vega.github.io/), [Vega-Lite](https://vega.github.io/vega-lite/), and [Altair](https://altair-viz.github.io/) projects.  If you're not familiar with these projects, it will be helpful to take a few minutes to browse their documentation before working through what VegaFusion adds.

### Vega(-Lite) Transforms and Signals
One powerful feature of the Vega visualization grammar is that it includes a rich collection of data manipulation functions called [transforms](https://vega.github.io/vega/docs/transforms/).  Transforms have functionality that is similar to that provided by SQL queries or Pandas DataFrame operations, but they are specifically designed to cover data preprocessing tasks that are useful in constructing data visualizations.

For additional flexibility, Vega also provides the concept of [signals](https://vega.github.io/vega/docs/signals/). These are scalar variables that can be constructed using the [Vega expression language](https://vega.github.io/vega/docs/expressions/), which is a subset of JavaScript.  Transforms can accept and produce signals.

There are two significant advantages to having data transformations and signals included in a visualization specification.  First, it makes it possible for a visualization to accept raw data files as input and then perform its own data cleaning and manipulation.  This often removes the need to generate temporary intermediary data files.  Second, it enables higher-level libraries like Vega-Lite to automate the creation of rich interactive visualizations with features like [cross filtering](https://vega.github.io/vega-lite/examples/interactive_layered_crossfilter.html) and [drill down](https://altair-viz.github.io/gallery/select_detail.html).

## Motivation for VegaFusion
Vega makes it possible to create declarative JSON specifications for rich interactive visualizations that are fully self-contained. They can run entirely in a web browser without requiring access to an external database or a Python library like Pandas.

For datasets of a few thousand rows or fewer, this architecture results in extremely smooth and responsive interactivity. However, this architecture doesn't scale very well to datasets of hundreds of thousands of rows or more.  This is the problem that VegaFusion aims to solve.

## How VegaFusion works
VegaFusion currently has two components: The Planner and the Runtime.

### Planner
The Planner starts with an arbitrary Vega specification (typically generated by Vega-Lite, but this is not a requirement). The Planner's job is to partition the specification into two valid Vega specifications, one that will execute in the browser with Vega.js, and one that will execute on the server with the VegaFusion Runtime.

VegaFusion does not (yet) provide full coverage of all of Vega's transforms and all of the features of the Vega expression language.  The planner uses information about which transforms and expression functions are supported to make decisions about which parts of the original vega specification can be included in the resulting server specification.  The advantage of this approach is that VegaFusion can accept any Vega specification, and as more support is added over time, more of the input specification will be eligible for inclusion in the server specification.

Along with the client and server specifications, the planner also creates a communication plan.  The communication plan is a specification of the datasets and signals that must be passed from server to client, and from client to server in order for the interactive behavior of the original specification to be preserved.

After planning, the client specification is evaluated by the Vega JavaScript library while the server specification is evaluated by the VegaFusion Runtime.

### Runtime
After planning, the server specification is compiled into a VegaFusion specific task graph specification.  The job of the runtime is to calculate the value of requested nodes within a task graph specification.

A task graph specification includes the values of the root nodes (those with no parents), but it does not include the values of any of the interior nodes (those with parents).  Each node in the task graph is a pure function of the values of its parents.  This enables the Runtime to calculate the value of any node in the Task graph from the specification.  The Runtime uses fingerprinting and caching to avoid repeated calculations of the same nodes.

### Inspecting the planner solution
To inspect how the planner partitioned the Vega specification, wrap the chart in a `VegaFusionWidget` and display it.

```python
import altair as alt
from vega_datasets import data

source = data.seattle_weather()
brush = alt.selection(type='interval', encodings=['x'])

bars = alt.Chart().mark_bar().encode(
    x='month(date):O',
    y='mean(precipitation):Q',
    opacity=alt.condition(brush, alt.OpacityValue(1), alt.OpacityValue(0.7)),
).add_selection(
    brush
)

line = alt.Chart().mark_rule(color='firebrick').encode(
    y='mean(precipitation):Q',
    size=alt.SizeValue(3)
).transform_filter(
    brush
)

widget = vf.VegaFusionWidget(alt.layer(bars, line, data=source))
widget
```
https://user-images.githubusercontent.com/15064365/148408648-43a5cfd0-b0d8-456e-a77a-dd344d8d07df.mov

Then print out the value of the following widget properties:

 - `widget.spec`: This is the Vega-Lite specification created by Altair
 - `widget.full_vega_spec`: This is the Vega specification produced by Vega-Lite
 - `widget.server_vega_spec`: This is the portion of the full Vega spec that was planned to run on the server (The Python kernel in case)
 - `widget.client_vega_spec`: This is the portion of the full Vega spec that was planned to run on the client, being rendered by Vega.js
 - `widget.comm_plan`: This is the specification of which signals and datasets must be transfered between the client and server in order to preserve the interactive behavior of the original specification.

## VegaFusion Architecture
The Planner runs entirely in the browser.  The responsibilities of the Runtime are split between the client and the server.  The client portion of the runtime is responsible for compiling the server specification into an efficient task graph representation.  As the root values of the task graph change (typically in response to user interaction events), the client runtime traverses the graph and compares the nodes in the traversal with the communication plan to determine which node values to request from the server.

The client runtime serializes the task graph and the query node ids and sends them as a request to the server runtime.  The server runtime receives the request, and checks whether the requested nodes are already available in its cache. If not it recursively traverses the ancestors of the query nodes until it finds a node value that is in the cache, or an inline root value of the Task Graph specification. It then traverses back down the task graph, evaluating the task functions as it goes. These newly computed task values are stored in the cache, and finally the requested node values are sent to the client.

### Architecture Advantages
This architecture is more complex than what would be required for only the Jupyter Widget scenario.  A characteristic of Jupyter Widgets is that there is a one-to-one correspondence between the widget state stored in the browser and the state stored on the server. So it would be fine for the task graph specification and the task graph values to be one data structure that is mirrored between the client and server.

This was actually the initial design. But it was soon apparent that this approach would not scale well to support future client server scenarios where one server process needs to support many clients. For example, when VegaFusion eventually supports Dash and custom client server configurations.  In these scenarios, it's not desirable for the server to maintain the full state of every visualization for every user.  This is especially wasteful when many users are viewing nearly identical visualizations.

With the VegaFusion runtime architecture, the server memory usage is independent of the number of simultaneous clients.  Each client request contains the full specification of the task graph, so the server doesn't need to remember the exact previous state of a client.  At the same time, it would be very inefficient if the server always had to compute every value in the full task graph on each request.  This inefficiency is addressed with precise caching.  The caching is "precise" in that each node of the task graph has a cache key that is generated from both its internal specification and that of all of its parents.  This means that common subgraphs across requests will have a shared cache entry even if the downstream tasks are different.

Another advantage of the approach is that a single client can freely change its task graph without having to notify the server runtime.  For example, a Vega editor backed by VegaFusion could send a slightly different task graph to the server as the spec is modified, but the cache will remain valid for the portions of the task graph that were not modified.  This effectively provides a hot reload capability.

## VegaFusion technology stack
VegaFusion uses a fairly diverse technology stack. The planner and runtime are both implemented in Rust.

In the context of `vegafusion-jupyter`, both the Planner and the client portion of the Runtime are compiled to WebAssembly using [wasm-pack](https://github.com/rustwasm/wasm-pack) and wrapped in a TypeScript API using [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen).  This TypeScript API is used to integrate the WebAssembly library into the VegaFusion Jupyter Widget.

The server portion of the Runtime is wrapped in a Python API using [PyO3](https://github.com/PyO3/pyo3), resulting in the `vegafusion-python` package.  The `vegafusion-jupyter` package is used to integrate vegafusion-python with Altair and the Python portion of VegaFusion Jupyter Widget.

The Task Graph specifications are defined as protocol buffer messages. The [prost](https://github.com/tokio-rs/prost) library is used to generate Rust data structures from these protocol buffer messages.  When Arrow tables appear as task graph root values, they are serialized inside the protocol buffer specification using the [Apache Arrow IPC format](https://arrow.apache.org/docs/format/Columnar.html#serialization-and-interprocess-communication-ipc).  The binary representation of the task graph protocol buffer message is what is transferred across the Jupyter Comms protocol.

<img width="749" alt="VegaFusion Jupyter Architecture Diagram" src="https://user-images.githubusercontent.com/15064365/148417030-19420ef2-50de-40cf-bd42-c39e1147049c.png">

## DataFusion integration
Apache Arrow DataFusion is an SQL compatible query engine that integrates with the Rust implementation of Apache Arrow.  VegaFusion uses DataFusion to implement many of the Vega transforms, and it compiles the Vega expression language directly into the DataFusion expression language.  In addition to being really fast, a particularly powerful characteristic of DataFusion is that it provides many interfaces that can be extended with your own custom Rust logic.  For example, VegaFusion defines many custom UDFs that are designed to implement the precise semantics of the Vega expression language and the Vega expression functions.

# License
At least until a sustainable funding model is established, VegaFusion will be developed under the [AGPL license](https://www.gnu.org/licenses/agpl-3.0.en.html).  This is a copy-left license in the GPL family of licenses. As with all [OSI approved licenses](https://opensource.org/licenses/alphabetical), there are no restrictions on what code licensed under the AGPL can be used for. However, the requirements for what must be shared publicly are greater than for licenses that are more commonly used in the Python ecosystem like [Apache-2](https://opensource.org/licenses/Apache-2.0), [MIT](https://opensource.org/licenses/MIT), and [BSD-3](https://opensource.org/licenses/BSD-3-Clause).

Contributors are asked to sign a Contributor License Agreement that allows their contribution to be re-licensed in the future. For example, the project could be re-licensed to one of the more permissive licenses above, or it could be dual licensed with a commercial license as a means to raise funds. 

# About the name
There are two reasons I chose the name VegaFusion
 - It's a nod to the [Apache Arrow DataFusion](https://github.com/apache/arrow-datafusion) library which is used to implement many of the supported Vega transforms
 - Vega and Altair are named after stars, and stars are powered by nuclear fusion.

# Building VegaFusion
If you're interested in building VegaFusion from source, see [BUILD.md]("./BUILD.md")

# Roadmap
Supporting serverside acceleration for Altair in Jupyter was chosen as the first MVP, but there are a lot of exciting ways that VegaFusion can be extended in the future.  Stay tuned for a full roadmap.

# Related projects
There are a few related projects that have some overlap with the goals of VegaFusion

## [`altair-transform`](https://github.com/altair-viz/altair-transform)
`altair-transform` is a Python library created by Jake Vanderplas, one of the creators of Altair. It consists of a pandas implementations of most of the Vega expression language and Vega Transforms that are available through Altair. It supports two main use cases:

 1. [Extracting Data](https://github.com/altair-viz/altair-transform#example-extracting-data): Given a Chart with transforms, `altair-transform` can be used to construct a Pandas DataFrame representing the result of the Chart's transforms, and thus the input to the Chart's mark.
 2. [Pre-Aggregating Large Datasets](https://github.com/altair-viz/altair-transform#example-pre-aggregating-large-datasets): Given a Chart with transforms, altair-transform can be used to evaluate the transforms and create a new Chart instance that refers only to this evaluated dataset. For aggregation charts like histograms, this can result in a much smaller dataset being transferred to the browser.
 
Both of these use cases are on the VegaFusion roadmap, but neither are implemented yet.  altair-transform does not support evaluating transforms on the server in interactive workflows like linked histogram brushing, which is an initial focus of VegaFusion. 

## [`ibis-vega-transform`](https://github.com/Quansight/ibis-vega-transform)
`ibis-vega-transform` is a Python library and JupyterLab extension developed by [Quantsite](https://www.quansight.com/). It translates pipelines of Vega transforms into [ibis](https://ibis-project.org/) query expressions, which can then be evaluated with a variety of ibis database backends (in particular, OmniSci). 

The JupyterLab extension makes two-way communication between the browser and the Python kernel possible, and this is used to support interactive visualizations like histogram brushing.

In contrast to the Planner approach used by VegaFusion, `ibis-vega-transform` replaces pipelines of Vega transforms with a custom transform type and then registers a JavaScript handler for this custom transform type.  This JavaScript handler then uses Jupyter Comms to communicate with the Python portion of the library. The Python library converts the requested Vega transforms into an ibis query, evaluates the query, and sends the resulting dataset back to the browser using a Jupyter Comm.

An advantage of this approach is that the Vega JavaScript library remains in control of the entire specification so the external `ibis-vega-transform` library does not need to maintain an independent task graph in order to support interactivity.  A downside of this approach is that the result of every transform pipeline must be sent back to the client and be stored in the Vega data flow graph.  Often times this is not a problem, because the transform pipeline includes an aggregation stage that significantly reduces the dataset size.  However, sometimes the result of a transform pipeline is quite large, but it is only used as input to other transform pipelines.  In this case, it is advantageous to keep the large intermediary result cached on the server and to not send it to the client at all.  This use case is one of the reasons that VegaFusion uses the Planner+Runtime architecture described above.

For the MVP, VegaFusion implements all of its transform logic in the Python process (with efficient multi-threading) and has no capability to connect to external data providers like databases.  This is certainly a desirable capability, and is on the VegaFusion roadmap.  Perhaps there will be a way to collaborate with the `ibis-vega-transform` project in the future to share a collection of ibis implementations of Vega transforms.