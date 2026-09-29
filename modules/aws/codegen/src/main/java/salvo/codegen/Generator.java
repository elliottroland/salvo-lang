package salvo.codegen;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.TreeMap;
import java.util.TreeSet;

import software.amazon.smithy.aws.traits.ServiceTrait;
import software.amazon.smithy.kotlin.codegen.core.NamingKt;
import software.amazon.smithy.kotlin.codegen.utils.CaseUtilsKt;
import software.amazon.smithy.model.Model;
import software.amazon.smithy.model.knowledge.TopDownIndex;
import software.amazon.smithy.model.shapes.BlobShape;
import software.amazon.smithy.model.shapes.BooleanShape;
import software.amazon.smithy.model.shapes.ByteShape;
import software.amazon.smithy.model.shapes.DoubleShape;
import software.amazon.smithy.model.shapes.EnumShape;
import software.amazon.smithy.model.shapes.FloatShape;
import software.amazon.smithy.model.shapes.IntegerShape;
import software.amazon.smithy.model.shapes.ListShape;
import software.amazon.smithy.model.shapes.LongShape;
import software.amazon.smithy.model.shapes.MapShape;
import software.amazon.smithy.model.shapes.MemberShape;
import software.amazon.smithy.model.shapes.OperationShape;
import software.amazon.smithy.model.shapes.ServiceShape;
import software.amazon.smithy.model.shapes.Shape;
import software.amazon.smithy.model.shapes.ShapeId;
import software.amazon.smithy.model.shapes.ShortShape;
import software.amazon.smithy.model.shapes.StringShape;
import software.amazon.smithy.model.shapes.StructureShape;
import software.amazon.smithy.model.traits.DefaultTrait;
import software.amazon.smithy.model.traits.DocumentationTrait;
import software.amazon.smithy.model.traits.RequiredTrait;
import software.amazon.smithy.model.traits.StreamingTrait;
import software.amazon.smithy.model.traits.TitleTrait;

/**
 * Walks one service's selected operations and renders four files:
 *
 * <ul>
 *   <li>{@code <module>.sv} — the Salvo surface: enums (unions of unit tags
 *       namespaced under the type, plus an open {@code Unknown} arm), structures,
 *       the service's error union, the effect, and a recording fake;</li>
 *   <li>{@code <module>/host.sv} — the {@code threadsafe platform handler} over
 *       the platform's SDK;</li>
 *   <li>{@code platform/<module>/host.rs} and {@code .kt} — the host glue, which
 *       converts between the Salvo values and the SDK's, runs each call on the
 *       SDK's own async machinery and completes the {@code Reply} from there.</li>
 * </ul>
 *
 * <p>Anything the model uses that this generator does not map yet — documents,
 * event streams, {@code @streaming} blobs, timestamps, unions, big numbers —
 * stops generation with an error naming the shape, rather than emitting
 * something approximate.
 */
final class Generator {
    private final Model model;
    private final Settings settings;
    private final ServiceShape service;
    private final List<OperationShape> operations = new ArrayList<>();
    /** Structures in first-reached order, with the Salvo name each renders as. */
    private final Map<ShapeId, String> structs = new LinkedHashMap<>();
    /** Enums (as value types) in first-reached order. */
    private final Map<ShapeId, String> enums = new LinkedHashMap<>();
    /** Every modeled error of the selected operations, by Salvo name. */
    private final TreeSet<String> errorNames = new TreeSet<>();
    private final Map<String, StructureShape> errorShapes = new TreeMap<>();
    private final Set<ShapeId> inputs = new LinkedHashSet<>();
    private final Set<ShapeId> outputs = new LinkedHashSet<>();

    Generator(Model model, Settings settings) {
        this.model = model;
        this.settings = settings;
        this.service = model.expectShape(settings.service(), ServiceShape.class);
    }

    // ================================================================ walk ===

    Map<String, String> generate() {
        Set<OperationShape> all = TopDownIndex.of(model).getContainedOperations(service);
        for (String name : settings.operations()) {
            OperationShape op = all.stream()
                    .filter(o -> o.getId().getName().equals(name))
                    .findFirst()
                    .orElseThrow(() -> fail(name, "is not an operation of " + service.getId()));
            operations.add(op);
        }
        for (OperationShape op : operations) {
            input(op).ifPresent(s -> {
                inputs.add(s.getId());
                structs.put(s.getId(), op.getId().getName() + "Input");
                walkStruct(s);
            });
            output(op).ifPresent(s -> {
                outputs.add(s.getId());
                structs.put(s.getId(), op.getId().getName() + "Output");
                walkStruct(s);
            });
            for (ShapeId e : op.getErrors(service)) {
                StructureShape es = model.expectShape(e, StructureShape.class);
                errorNames.add(e.getName());
                errorShapes.put(e.getName(), es);
                walkStruct(es);
            }
        }
        Map<String, String> files = new LinkedHashMap<>();
        String path = settings.module().replace('.', '/');
        files.put(path + ".sv", salvoSurface());
        files.put(path + "/host.sv", salvoHost());
        files.put("platform/" + path + "/host.rs", rustGlue());
        files.put("platform/" + path + "/host.kt", kotlinGlue());
        return files;
    }

    private Optional<StructureShape> input(OperationShape op) {
        return op.getInput().filter(id -> !id.toString().equals("smithy.api#Unit"))
                .map(id -> model.expectShape(id, StructureShape.class));
    }

    private Optional<StructureShape> output(OperationShape op) {
        return op.getOutput().filter(id -> !id.toString().equals("smithy.api#Unit"))
                .map(id -> model.expectShape(id, StructureShape.class));
    }

    private void walkStruct(StructureShape s) {
        for (MemberShape m : s.getAllMembers().values()) {
            walkType(model.expectShape(m.getTarget()), m);
        }
    }

    private void walkType(Shape t, MemberShape via) {
        if (t.hasTrait(StreamingTrait.class)) {
            throw fail(via.getId().toString(), "is a streaming member, which this generator does not map yet");
        }
        if (t instanceof EnumShape e) {
            enums.putIfAbsent(e.getId(), e.getId().getName());
        } else if (t instanceof StructureShape s) {
            if (!structs.containsKey(s.getId()) && !s.hasTrait("smithy.api#error")) {
                structs.put(s.getId(), s.getId().getName());
                walkStruct(s);
            }
        } else if (t instanceof ListShape l) {
            walkType(model.expectShape(l.getMember().getTarget()), l.getMember());
        } else if (t instanceof MapShape mp) {
            Shape key = model.expectShape(mp.getKey().getTarget());
            if (!(key instanceof StringShape)) {
                throw fail(mp.getId().toString(), "has a key that is not a string");
            }
            walkType(model.expectShape(mp.getValue().getTarget()), mp.getValue());
        } else if (!(t instanceof StringShape || t instanceof IntegerShape || t instanceof ShortShape
                || t instanceof ByteShape || t instanceof LongShape || t instanceof BooleanShape
                || t instanceof FloatShape || t instanceof DoubleShape || t instanceof BlobShape)) {
            throw fail(via.getId().toString(), "targets a " + t.getType() + ", which this generator does not map yet");
        }
    }

    private static IllegalStateException fail(String what, String why) {
        return new IllegalStateException("salvo-client-codegen: `" + what + "` " + why);
    }

    // =============================================================== names ===

    private static final Set<String> SALVO_KEYWORDS = Set.of(
            "fn", "let", "struct", "qualifier", "effect", "handler", "params", "type", "intrinsic",
            "platform", "import", "as", "of", "with", "canbe", "provenance", "refn", "rename", "is",
            "if", "elif", "else", "when", "while", "for", "in", "return", "break", "continue", "state",
            "use", "try", "true", "false", "proj", "once", "linear", "holds", "export", "copy");
    private static final Set<String> RUST_KEYWORDS = Set.of(
            "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do",
            "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in", "let",
            "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return",
            "static", "struct", "trait", "true", "try", "type", "typeof", "unsafe", "unsized", "use",
            "virtual", "where", "while", "yield");
    private static final Set<String> KOTLIN_KEYWORDS = Set.of(
            "as", "break", "class", "continue", "do", "else", "false", "for", "fun", "if", "in",
            "interface", "is", "null", "object", "package", "return", "super", "this", "throw", "true",
            "try", "typealias", "typeof", "val", "var", "when", "while");

    /** smithy-rs's snake case: the shared word-boundary splitter, lowered and joined. */
    static String snake(String name) {
        return String.join("_", CaseUtilsKt.splitOnWordBoundaries(name)).toLowerCase();
    }

    private static String pascal(String name) {
        return CaseUtilsKt.toPascalCase(name);
    }

    /** A member's Salvo field name: snake case, a Salvo keyword suffixed with `_`. */
    private static String salvoField(MemberShape m) {
        String s = snake(m.getMemberName());
        return SALVO_KEYWORDS.contains(s) ? s + "_" : s;
    }

    /** A Salvo name as the Rust emitter spells it (`rs_ident`). */
    private static String rsIdent(String salvo) {
        return RUST_KEYWORDS.contains(salvo) ? "r#" + salvo : salvo;
    }

    /** A Salvo name as the Kotlin emitter spells it (`kt_ident`). */
    private static String ktIdent(String salvo) {
        return KOTLIN_KEYWORDS.contains(salvo) ? "`" + salvo + "`" : salvo;
    }

    /** The SDK's Rust accessor for a member. */
    private static String rsAccessor(MemberShape m) {
        String s = snake(m.getMemberName());
        return RUST_KEYWORDS.contains(s) ? "r#" + s : s;
    }

    /** The SDK's Kotlin property for a member — smithy-kotlin's own rule. */
    private static String ktMember(MemberShape m) {
        return NamingKt.defaultName(m);
    }

    private String rsCrate() {
        return settings.rustCrate();
    }

    private String ktModel() {
        return settings.kotlinPackage() + ".model";
    }

    private String ktClient() {
        String sdkId = service.getTrait(ServiceTrait.class).map(ServiceTrait::getSdkId)
                .orElse(service.getId().getName());
        return settings.kotlinPackage() + "." + NamingKt.clientName(sdkId) + "Client";
    }

    /** A structure's SDK type, Rust. */
    private String rsSdkStruct(ShapeId id) {
        for (OperationShape op : operations) {
            if (output(op).map(Shape::getId).filter(id::equals).isPresent()) {
                return rsCrate() + "::operation::" + snake(op.getId().getName()) + "::"
                        + op.getId().getName() + "Output";
            }
        }
        if (model.expectShape(id).hasTrait("smithy.api#error")) {
            return rsCrate() + "::types::error::" + id.getName();
        }
        return rsCrate() + "::types::" + id.getName();
    }

    /** A structure's SDK type, Kotlin (operation shapes are renamed Request/Response). */
    private String ktSdkStruct(ShapeId id) {
        for (OperationShape op : operations) {
            if (input(op).map(Shape::getId).filter(id::equals).isPresent()) {
                return ktModel() + "." + op.getId().getName() + "Request";
            }
            if (output(op).map(Shape::getId).filter(id::equals).isPresent()) {
                return ktModel() + "." + op.getId().getName() + "Response";
            }
        }
        return ktModel() + "." + id.getName();
    }

    private String effectOp(OperationShape op) {
        return snake(op.getId().getName());
    }

    private String errorUnion() {
        return settings.effect() + "Error";
    }

    // =============================================================== types ===

    /** A member's Salvo type, without optionality. */
    private String salvoType(Shape t) {
        if (t instanceof EnumShape e) return enums.get(e.getId());
        if (t instanceof StructureShape s) {
            return s.hasTrait("smithy.api#error") ? s.getId().getName() : structs.get(s.getId());
        }
        if (t instanceof ListShape l) return "List<" + salvoType(model.expectShape(l.getMember().getTarget())) + ">";
        if (t instanceof MapShape m) return "Map<Str, " + salvoType(model.expectShape(m.getValue().getTarget())) + ">";
        if (t instanceof StringShape) return "Str";
        if (t instanceof IntegerShape || t instanceof ShortShape || t instanceof ByteShape) return "Int";
        if (t instanceof LongShape) return "Long";
        if (t instanceof BooleanShape) return "Bool";
        if (t instanceof FloatShape) return "Float";
        if (t instanceof DoubleShape) return "Double";
        if (t instanceof BlobShape) return "Bytes";
        throw fail(t.getId().toString(), "has no Salvo type");
    }

    /** Whether a member is present on every value: required, or defaulted. */
    private static boolean present(MemberShape m) {
        return m.hasTrait(RequiredTrait.class) || defaultOf(m) != null;
    }

    /** A literal Salvo default for a `@default` member, or null. */
    private static String defaultOf(MemberShape m) {
        return m.getTrait(DefaultTrait.class).map(d -> {
            var n = d.toNode();
            if (n.isNullNode()) return null;
            if (n.isBooleanNode()) return String.valueOf(n.expectBooleanNode().getValue());
            if (n.isNumberNode()) return n.expectNumberNode().getValue().toString();
            if (n.isStringNode()) return "\"" + n.expectStringNode().getValue().replace("\"", "\\\"") + "\"";
            throw fail(m.getId().toString(), "has a non-scalar @default, which this generator does not map yet");
        }).orElse(null);
    }

    private Shape target(MemberShape m) {
        return model.expectShape(m.getTarget());
    }

    // ================================================================ docs ===

    /** The first paragraph of a shape's documentation, as plain text. */
    private static String docText(Shape s) {
        String html = s.getTrait(DocumentationTrait.class).map(DocumentationTrait::getValue).orElse("");
        int end = html.indexOf("</p>");
        if (end >= 0) html = html.substring(0, end);
        String text = html.replaceAll("</?(p|li|ul|ol|br|dd|dt|dl)[^>]*>", " ").replaceAll("<[^>]+>", "")
                .replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"")
                .replace("&#39;", "'").replace("&amp;", "&")
                .replaceAll("\\s+", " ").replaceAll(" ([.,;:)])", "$1").trim();
        return text;
    }

    /** `// `-comment lines at [indent], wrapped to 80 columns. */
    private static String comment(String text, String indent) {
        if (text.isEmpty()) return "";
        StringBuilder out = new StringBuilder();
        StringBuilder line = new StringBuilder();
        int width = 80 - indent.length() - 3;
        for (String word : text.split(" ")) {
            if (line.length() > 0 && line.length() + 1 + word.length() > width) {
                out.append(indent).append("// ").append(line).append('\n');
                line.setLength(0);
            }
            if (line.length() > 0) line.append(' ');
            line.append(word);
        }
        if (line.length() > 0) out.append(indent).append("// ").append(line).append('\n');
        return out.toString();
    }

    private String header(String what) {
        return "// Generated by `salvo-client-codegen` from `" + settings.modelFile() + "`; do not\n"
                + "// edit — regenerate with `./gradlew generate` in `modules/aws/codegen`.\n";
    }

    // ======================================================= Salvo surface ===

    private String salvoSurface() {
        StringBuilder out = new StringBuilder();
        String title = service.getTrait(TitleTrait.class).map(TitleTrait::getValue).orElse(settings.effect());
        out.append(comment(title + ", as a Salvo effect: every operation takes a `Reply` and returns at once, "
                + "answering when the service does [platform-reply].", ""));
        out.append("//\n");
        out.append(comment(docText(service), ""));
        out.append("//\n");
        out.append(comment("The host implementation is `" + settings.module() + ".host`'s `Host"
                + settings.effect() + "`; `Fake" + settings.effect() + "` below is a recording double "
                + "that needs no SDK.", ""));
        out.append("//\n");
        out.append(header(""));
        out.append("\nimport aws\n");

        for (Map.Entry<ShapeId, String> e : enums.entrySet()) {
            out.append('\n').append(salvoEnum(model.expectShape(e.getKey(), EnumShape.class)));
        }
        for (Map.Entry<ShapeId, String> s : structs.entrySet()) {
            out.append('\n').append(salvoStruct(model.expectShape(s.getKey(), StructureShape.class), s.getValue()));
        }
        for (Map.Entry<String, StructureShape> e : errorShapes.entrySet()) {
            out.append('\n').append(salvoStruct(e.getValue(), e.getKey()));
        }
        out.append('\n');
        out.append(comment("Everything an operation of [" + settings.effect() + "] can answer with instead of "
                + "its output: the errors the model names, and [AwsError] for everything else.", ""));
        List<String> arms = new ArrayList<>(errorNames);
        arms.add("AwsError");
        out.append("export type ").append(errorUnion()).append(" = ").append(String.join("\n    | ", arms)).append('\n');

        // The effect.
        out.append('\n');
        out.append(comment(title + ". Every member hands its [reply] to the provider and returns at once; "
                + "the answer is the operation's output or a `Checked` [" + errorUnion() + "].", ""));
        out.append("export effect ").append(settings.effect()).append(" {\n");
        boolean first = true;
        for (OperationShape op : operations) {
            if (!first) out.append('\n');
            first = false;
            out.append(comment(docText(op), "    "));
            out.append("    ").append(effectSignature(op)).append('\n');
        }
        out.append("}\n");

        // The recording fake.
        String calls = settings.effect() + "Calls";
        out.append('\n');
        out.append(comment("What a [Fake" + settings.effect() + "] was asked, in order: one operation name per call.", ""));
        out.append("export effect ").append(calls).append(" {\n");
        out.append("    // The names of the operations called so far (`create_queue`, …).\n");
        out.append("    fn calls() -> List<Str>\n}\n");
        out.append('\n');
        out.append(comment("A recording double for [" + settings.effect() + "]: every call is noted by operation "
                + "name — read them through [" + calls + "] — and answered with an empty output where the "
                + "operation's output has no required members, otherwise with `AwsError { code: \"NotStubbed\" }`. "
                + "A test that needs real answers implements [" + settings.effect() + "] itself.", ""));
        out.append("export handler Fake").append(settings.effect()).append("() of ").append(settings.effect())
                .append(", ").append(calls).append(" {\n");
        out.append("    recorded: Mut List<Str> = mut_list_of()\n");
        for (OperationShape op : operations) {
            out.append('\n');
            out.append("    ").append(effectSignature(op).replace("\n    =>", " =>")).append(" {\n");
            out.append("        recorded.add(\"").append(effectOp(op)).append("\")\n");
            out.append("        reply.send(").append(fakeAnswer(op)).append(")\n");
            out.append("    }\n");
        }
        out.append("\n    fn calls() -> List<Str> {\n        return copy(recorded)\n    }\n}\n");
        return out.toString();
    }

    private String effectSignature(OperationShape op) {
        StringBuilder sig = new StringBuilder("fn ").append(effectOp(op)).append('(');
        List<String> moved = new ArrayList<>();
        input(op).ifPresent(s -> {
            sig.append("input: ").append(structs.get(s.getId())).append(", ");
            moved.add("!input");
        });
        sig.append("reply: Reply<").append(replyPayload(op)).append(">) -> None\n    => ");
        moved.add("!reply");
        sig.append(String.join(", ", moved));
        return sig.toString();
    }

    private String replyPayload(OperationShape op) {
        String out = output(op).map(s -> structs.get(s.getId())).orElse("None");
        return "Ok " + out + " | Err Checked<" + errorUnion() + ">";
    }

    private String fakeAnswer(OperationShape op) {
        Optional<StructureShape> out = output(op);
        if (out.isEmpty()) return "ok(None)";
        boolean stubbable = out.get().getAllMembers().values().stream().noneMatch(Generator::present);
        if (stubbable) return "ok(" + structs.get(out.get().getId()) + " {})";
        return "err(checked<" + errorUnion() + ">(AwsError { code: \"NotStubbed\", message: \"Fake"
                + settings.effect() + " cannot answer " + effectOp(op) + "\" }))";
    }

    private List<String> variants(EnumShape e) {
        List<String> out = new ArrayList<>();
        for (String member : e.getEnumValues().keySet()) out.add(pascal(member));
        return out;
    }

    private String unknownVariant(EnumShape e) {
        return variants(e).contains("Unknown") ? "UnknownValue" : "Unknown";
    }

    private String salvoEnum(EnumShape e) {
        String name = enums.get(e.getId());
        StringBuilder out = new StringBuilder();
        out.append(comment(docText(e).isEmpty() ? name + ", one arm per value the model names." : docText(e), ""));
        out.append("//\n// Open, as Smithy enums are: a value this model does not name arrives as `")
                .append(name).append('.').append(unknownVariant(e)).append("`.\n");
        List<String> arms = new ArrayList<>();
        for (String v : variants(e)) arms.add(name + "." + v);
        arms.add(name + "." + unknownVariant(e));
        out.append("export type ").append(name).append(" = ").append(String.join("\n    | ", arms)).append('\n');
        for (Map.Entry<String, String> v : e.getEnumValues().entrySet()) {
            out.append("// `\"").append(v.getValue()).append("\"`\n");
            out.append("export struct ").append(name).append('.').append(pascal(v.getKey())).append(" {}\n");
        }
        out.append("// A value this model does not name.\n");
        out.append("export struct ").append(name).append('.').append(unknownVariant(e)).append(" { value: Str }\n");
        return out.toString();
    }

    private String salvoStruct(StructureShape s, String name) {
        StringBuilder out = new StringBuilder();
        out.append(comment(docText(s), ""));
        List<MemberShape> members = new ArrayList<>(s.getAllMembers().values());
        if (members.isEmpty()) {
            out.append("export struct ").append(name).append(" {}\n");
            return out.toString();
        }
        out.append("export struct ").append(name).append(" {\n");
        for (int i = 0; i < members.size(); i++) {
            MemberShape m = members.get(i);
            out.append(comment(docText(m), "    "));
            String ty = salvoType(target(m));
            out.append("    ").append(salvoField(m)).append(": ");
            String def = defaultOf(m);
            if (m.hasTrait(RequiredTrait.class)) {
                out.append(ty);
            } else if (def != null) {
                out.append(ty).append(" = ").append(def);
            } else {
                out.append(ty).append("? = None");
            }
            out.append(i + 1 < members.size() ? ",\n" : "\n");
        }
        out.append("}\n");
        return out.toString();
    }

    private String salvoHost() {
        StringBuilder out = new StringBuilder();
        out.append(comment("The host's " + service.getTrait(TitleTrait.class).map(TitleTrait::getValue)
                .orElse(settings.effect()) + ": [Host" + settings.effect() + "] implements `" + settings.module()
                + "`'s [" + settings.effect() + "] over aws-sdk-kotlin and aws-sdk-rust.", ""));
        out.append("//\n");
        out.append(comment("Its own module, as `fs.host` is: a program that fakes the service never reaches "
                + "the host glue, so it builds without the SDK.", ""));
        out.append("//\n");
        out.append(header(""));
        out.append("\nimport aws\nimport ").append(settings.module()).append("\n\n");
        out.append(comment("[" + settings.effect() + "] through the platform's AWS SDK, configured by [config]: "
                + "each call runs on the SDK's own asynchronous machinery (coroutines on the JVM, tokio on "
                + "Rust) and completes its reply from there, so no Salvo worker waits. `threadsafe`: the SDK "
                + "client is safe to share, so the instance is used from every pool with no lock.", ""));
        out.append("export threadsafe platform handler Host").append(settings.effect())
                .append("(config: AwsConfig) of ").append(settings.effect()).append('\n');
        return out.toString();
    }

    // ============================================================ Rust glue ===

    private String rsSalvoModule() {
        return "crate::" + settings.module().replace('.', '_');
    }

    /** A Salvo struct or enum name as the Rust emitter spells it (dot-names flattened). */
    private static String rsType(String salvoName) {
        return salvoName.replace(".", "");
    }

    private String rsErrorUnionType() {
        List<String> arms = new ArrayList<>();
        for (String e : errorNames) arms.add(rsType(e));
        arms.add("AwsError");
        return "Union" + arms.size() + "<" + String.join(", ", arms) + ">";
    }

    private String rsEnumUnion(EnumShape e) {
        String name = enums.get(e.getId());
        List<String> arms = new ArrayList<>();
        for (String v : variants(e)) arms.add(name + v);
        arms.add(name + unknownVariant(e));
        return "Union" + arms.size() + "<" + String.join(", ", arms) + ">";
    }

    /** Salvo → SDK, from a reference expression [ref] to the Salvo value. */
    private String rsToSdk(Shape t, String ref) {
        if (t instanceof EnumShape e) return "to_sdk_" + snake(enums.get(e.getId())) + "(" + ref + ")";
        if (t instanceof StructureShape s) return "to_sdk_" + snake(structs.get(s.getId())) + "(" + ref + ")";
        if (t instanceof ListShape l) {
            return ref + ".iter().map(|e| " + rsToSdk(target(l.getMember()), "e") + ").collect::<Vec<_>>()";
        }
        if (t instanceof MapShape m) {
            Shape key = target(m.getKey());
            String k = key instanceof EnumShape
                    ? rsCrate() + "::types::" + key.getId().getName() + "::from(k.as_str())"
                    : "k.clone()";
            return ref + ".iter().map(|(k, v)| (" + k + ", " + rsToSdk(target(m.getValue()), "v")
                    + ")).collect::<std::collections::HashMap<_, _>>()";
        }
        if (t instanceof StringShape) return ref + ".clone()";
        if (t instanceof BlobShape) return rsCrate() + "::primitives::Blob::new(" + ref + ".clone())";
        return "*" + ref;
    }

    /** SDK → Salvo, from a reference expression [ref] to the SDK value. */
    private String rsFromSdk(Shape t, String ref) {
        if (t instanceof EnumShape e) return "from_sdk_" + snake(enums.get(e.getId())) + "(" + ref + ")";
        if (t instanceof StructureShape s) return "from_sdk_" + snake(salvoType(s)) + "(" + ref + ")";
        if (t instanceof ListShape l) {
            return ref + ".iter().map(|e| " + rsFromSdk(target(l.getMember()), "e") + ").collect::<Vec<_>>()";
        }
        if (t instanceof MapShape m) {
            Shape key = target(m.getKey());
            String k = key instanceof EnumShape ? "k.as_str().to_string()" : "k.clone()";
            return "{ let mut __es: Vec<_> = " + ref + ".iter().map(|(k, v)| (" + k + ", "
                    + rsFromSdk(target(m.getValue()), "v") + ")).collect(); "
                    + "__es.sort_by(|a, b| a.0.cmp(&b.0)); "
                    + "crate::collections::SalvoMap::from_entries::<crate::collections::HostHash, "
                    + "crate::collections::HostEq, _>(__es) }";
        }
        if (t instanceof StringShape) return ref + ".to_string()";
        if (t instanceof BlobShape) return ref + ".as_ref().to_vec()";
        return "*" + ref;
    }

    private static boolean isPrimitive(Shape t) {
        return t instanceof IntegerShape || t instanceof ShortShape || t instanceof ByteShape
                || t instanceof LongShape || t instanceof BooleanShape || t instanceof FloatShape
                || t instanceof DoubleShape;
    }

    /** The Salvo value of one member, read from the SDK value [src] (a reference). */
    private String rsReadField(MemberShape m, String src) {
        Shape t = target(m);
        String acc = src + "." + rsAccessor(m) + "()";
        boolean present = present(m);
        if (t instanceof ListShape) {
            // smithy-rs answers `&[T]` for a list, empty when absent.
            if (present) return rsFromSdk(t, acc);
            return "{ let __s = " + acc + "; if __s.is_empty() { None } else { Some(" + rsFromSdk(t, "__s") + ") } }";
        }
        if (isPrimitive(t)) {
            return present ? rsFromSdk(t, "(&" + acc + ")") : acc + ".map(|x| " + rsFromSdk(t, "(&x)") + ")";
        }
        return present ? rsFromSdk(t, acc) : acc + ".map(|x| " + rsFromSdk(t, "x") + ")";
    }

    /** The `set_x(...)` argument for one member, from the Salvo struct [src] (a place). */
    private String rsSetArg(MemberShape m, String src) {
        Shape t = target(m);
        String field = src + "." + rsIdent(salvoField(m));
        if (m.hasTrait(RequiredTrait.class) || defaultOf(m) != null) {
            return "Some(" + rsToSdk(t, "(&" + field + ")") + ")";
        }
        return field + ".as_ref().map(|x| " + rsToSdk(t, "x") + ")";
    }

    private String rustGlue() {
        String crate = rsCrate();
        String effect = settings.effect();
        StringBuilder out = new StringBuilder();
        out.append("// Host implementation of `").append(settings.module()).append(".host`'s `Host").append(effect)
                .append("`,\n// over ").append(crate).append(" [platform-reply].\n//\n");
        out.append(header(""));
        out.append("\n#![allow(unused_imports, dead_code, deprecated)]\n");
        out.append("\nuse crate::aws::*;\nuse ").append(rsSalvoModule()).append("::*;\nuse ")
                .append(rsSalvoModule()).append("_host::*;\nuse crate::unions::*;\nuse crate::core_checked::Checked;\n");
        out.append("use ").append(crate).append("::error::ProvideErrorMetadata;\n\n");

        out.append("// `threadsafe platform handler Host").append(effect).append("`: the SDK client is cheap to\n")
                .append("// clone and safe to share, and each call runs as a task on the handler's own\n")
                .append("// tokio runtime, completing its reply from there.\n");
        out.append("pub struct Host").append(effect).append(" {\n    rt: tokio::runtime::Runtime,\n    client: ")
                .append(crate).append("::Client,\n}\n\n");
        out.append("impl Host").append(effect).append(" {\n    pub fn new(config: AwsConfig) -> Self {\n")
                .append("        let rt = tokio::runtime::Builder::new_multi_thread()\n")
                .append("            .enable_all()\n            .build()\n")
                .append("            .expect(\"a tokio runtime for the AWS SDK\");\n")
                .append("        let shared = rt.block_on(salvo_aws_config(&config));\n")
                .append("        let client = ").append(crate).append("::Client::new(&shared);\n")
                .append("        Self { rt, client }\n    }\n}\n\n");
        out.append(rustConfigLoader());

        out.append("impl ").append(rsSalvoModule()).append("::__Stateless_").append(effect).append(" for Host")
                .append(effect).append(" {\n");
        boolean first = true;
        for (OperationShape op : operations) {
            if (!first) out.append('\n');
            first = false;
            out.append(rustMember(op));
        }
        out.append("}\n");

        // Conversions.
        for (ShapeId id : enums.keySet()) out.append('\n').append(rustEnumConv(model.expectShape(id, EnumShape.class)));
        for (Map.Entry<ShapeId, String> s : structs.entrySet()) {
            StructureShape shape = model.expectShape(s.getKey(), StructureShape.class);
            if (!inputs.contains(s.getKey())) out.append('\n').append(rustFromSdk(shape, s.getValue()));
            if (!outputs.contains(s.getKey()) && !inputs.contains(s.getKey())) {
                out.append('\n').append(rustToSdk(shape, s.getValue()));
            }
        }
        for (Map.Entry<String, StructureShape> e : errorShapes.entrySet()) {
            out.append('\n').append(rustFromSdk(e.getValue(), e.getKey()));
        }
        for (OperationShape op : operations) out.append('\n').append(rustErrorConv(op));
        out.append(rustExpandHome());
        return out.toString();
    }

    private String rustMember(OperationShape op) {
        String crate = rsCrate();
        String name = effectOp(op);
        StringBuilder out = new StringBuilder();
        out.append("    fn ").append(name).append("(&self, ");
        if (input(op).isPresent()) {
            out.append("input: ").append(rsType(structs.get(input(op).get().getId()))).append(", ");
        }
        out.append("reply: crate::scheduler::SalvoReply) {\n");
        out.append("        let reply = reply.hosted();\n        let client = self.client.clone();\n");
        out.append("        let call = client.").append(name).append("()");
        input(op).ifPresent(in -> {
            for (MemberShape m : in.getAllMembers().values()) {
                out.append("\n            .set_").append(snake(m.getMemberName())).append('(')
                        .append(rsSetArg(m, "input")).append(')');
            }
        });
        out.append(";\n");
        out.append("        self.rt.spawn(async move {\n");
        String okType = output(op).map(s -> rsType(structs.get(s.getId()))).orElse("()");
        out.append("            let answer: Union2<").append(okType).append(", Checked<").append(rsErrorUnionType())
                .append(">> = match call.send().await {\n");
        String ok = output(op).map(s -> "from_sdk_" + snake(structs.get(s.getId())) + "(&out)").orElse("()");
        out.append("                Ok(out) => { let _ = &out; Union2::U1(").append(ok).append(") }\n");
        out.append("                Err(e) => Union2::U2(Checked { value: error_of_").append(name).append("(e) }),\n");
        out.append("            };\n            reply.send(answer);\n        });\n    }\n");
        return out.toString();
    }

    private String rustEnumConv(EnumShape e) {
        String crate = rsCrate();
        String name = enums.get(e.getId());
        String sdk = crate + "::types::" + e.getId().getName();
        String union = rsEnumUnion(e);
        int n = e.getEnumValues().size() + 1;
        StringBuilder to = new StringBuilder();
        to.append("fn to_sdk_").append(snake(name)).append("(v: &").append(union).append(") -> ").append(sdk).append(" {\n    match v {\n");
        StringBuilder from = new StringBuilder();
        from.append("fn from_sdk_").append(snake(name)).append("(v: &").append(sdk).append(") -> ").append(union)
                .append(" {\n    match v.as_str() {\n");
        int i = 1;
        for (Map.Entry<String, String> v : e.getEnumValues().entrySet()) {
            String arm = name + pascal(v.getKey());
            to.append("        Union").append(n).append("::U").append(i).append("(_) => ").append(sdk)
                    .append("::from(\"").append(v.getValue()).append("\"),\n");
            from.append("        \"").append(v.getValue()).append("\" => Union").append(n).append("::U").append(i)
                    .append('(').append(arm).append(" {}),\n");
            i++;
        }
        to.append("        Union").append(n).append("::U").append(n).append("(u) => ").append(sdk)
                .append("::from(u.value.as_str()),\n    }\n}\n");
        from.append("        other => Union").append(n).append("::U").append(n).append('(').append(name)
                .append(unknownVariant(e)).append(" { value: other.to_string() }),\n    }\n}\n");
        return to + "\n" + from;
    }

    private String rustFromSdk(StructureShape s, String name) {
        StringBuilder out = new StringBuilder();
        out.append("fn from_sdk_").append(snake(name)).append("(v: &").append(rsSdkStruct(s.getId())).append(") -> ")
                .append(rsType(name)).append(" {\n    ").append(rsType(name)).append(" {");
        List<MemberShape> members = new ArrayList<>(s.getAllMembers().values());
        if (members.isEmpty()) return out.append("}\n}\n").toString();
        out.append('\n');
        for (MemberShape m : members) {
            out.append("        ").append(rsIdent(salvoField(m))).append(": ").append(rsReadField(m, "v")).append(",\n");
        }
        out.append("    }\n}\n");
        return out.toString();
    }

    private String rustToSdk(StructureShape s, String name) {
        String sdk = rsSdkStruct(s.getId());
        StringBuilder out = new StringBuilder();
        out.append("fn to_sdk_").append(snake(name)).append("(v: &").append(rsType(name)).append(") -> ").append(sdk)
                .append(" {\n    let b = ").append(sdk).append("::builder()");
        boolean fallible = false;
        for (MemberShape m : s.getAllMembers().values()) {
            out.append("\n        .set_").append(snake(m.getMemberName())).append('(').append(rsSetArg(m, "v")).append(')');
            fallible |= m.hasTrait(RequiredTrait.class);
        }
        out.append(";\n    b.build()");
        if (fallible) out.append(".expect(\"every required member of ").append(name).append(" is set\")");
        out.append("\n}\n");
        return out.toString();
    }

    private String rustErrorConv(OperationShape op) {
        String crate = rsCrate();
        String opSnake = effectOp(op);
        String errEnum = crate + "::operation::" + opSnake + "::" + op.getId().getName() + "Error";
        List<String> all = new ArrayList<>(errorNames);
        int n = all.size() + 1;
        StringBuilder out = new StringBuilder();
        out.append("fn error_of_").append(opSnake).append("(e: ").append(crate).append("::error::SdkError<")
                .append(errEnum).append(">) -> ").append(rsErrorUnionType()).append(" {\n");
        out.append("    let text = format!(\"{}\", ").append(crate).append("::error::DisplayErrorContext(&e));\n");
        out.append("    match e.into_service_error() {\n");
        for (ShapeId err : op.getErrors(service)) {
            int at = all.indexOf(err.getName()) + 1;
            out.append("        ").append(errEnum).append("::").append(err.getName()).append("(x) => Union").append(n)
                    .append("::U").append(at).append("(from_sdk_").append(snake(err.getName())).append("(&x)),\n");
        }
        out.append("        other => Union").append(n).append("::U").append(n)
                .append("(AwsError { code: other.code().unwrap_or(\"SdkError\").to_string(), message: text }),\n");
        out.append("    }\n}\n");
        return out.toString();
    }

    private String rustConfigLoader() {
        return """
                // The SDK configuration an `AwsConfig` describes: the region, the credentials
                // it names, and the endpoint override when there is one.
                #[allow(deprecated)]
                async fn salvo_aws_config(config: &AwsConfig) -> aws_config::SdkConfig {
                    let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest())
                        .region(aws_config::Region::new(config.region.code.clone()));
                    match &config.credentials {
                        Union3::U1(p) => {
                            use aws_config::profile::profile_file::{ProfileFileKind, ProfileFiles};
                            let files = ProfileFiles::builder()
                                .with_file(ProfileFileKind::Credentials, salvo_expand_home(&p.path))
                                .build();
                            let provider = aws_config::profile::ProfileFileCredentialsProvider::builder()
                                .profile_files(files)
                                .profile_name(p.profile.clone())
                                .build();
                            loader = loader.credentials_provider(provider);
                        }
                        Union3::U2(_) => {
                            loader = loader.credentials_provider(
                                aws_config::environment::EnvironmentVariableCredentialsProvider::new(),
                            );
                        }
                        Union3::U3(_) => {}
                    }
                    if let Some(endpoint) = &config.endpoint {
                        loader = loader.endpoint_url(endpoint.clone());
                    }
                    loader.load().await
                }

                """;
    }

    private String rustExpandHome() {
        return """

                /// `~/` at the front of a path is the home directory, as a shell would read it.
                fn salvo_expand_home(path: &str) -> String {
                    match (path.strip_prefix("~/"), std::env::var("HOME")) {
                        (Some(rest), Ok(home)) => format!("{home}/{rest}"),
                        _ => path.to_string(),
                    }
                }
                """;
    }

    // ========================================================== Kotlin glue ===

    private String ktEnumUnion(EnumShape e) {
        String name = enums.get(e.getId());
        List<String> arms = new ArrayList<>();
        for (String v : variants(e)) arms.add(name + "." + v);
        arms.add(name + "." + unknownVariant(e));
        return "Union" + arms.size() + "<" + String.join(", ", arms) + ">";
    }

    private static String stars(int n) {
        return "<" + String.join(", ", java.util.Collections.nCopies(n, "*")) + ">";
    }

    /** Salvo → SDK, from a non-null expression [v]; [d] numbers nested lambda parameters. */
    private String ktToSdk(Shape t, String v, int d) {
        if (t instanceof EnumShape e) return "toSdk" + enums.get(e.getId()) + "(" + v + ")";
        if (t instanceof StructureShape s) return "toSdk" + structs.get(s.getId()) + "(" + v + ")";
        if (t instanceof ListShape l) {
            return v + ".map { e" + d + " -> " + ktToSdk(target(l.getMember()), "e" + d, d + 1) + " }";
        }
        if (t instanceof MapShape m) {
            Shape key = target(m.getKey());
            String k = key instanceof EnumShape ? ktModel() + "." + key.getId().getName() + ".fromValue(k" + d + ")" : "k" + d;
            return v + ".entries.associate { (k" + d + ", v" + d + ") -> " + k + " to "
                    + ktToSdk(target(m.getValue()), "v" + d, d + 1) + " }";
        }
        if (t instanceof BlobShape) return v + ".toByteArray()";
        return v;
    }

    /** SDK → Salvo, from a non-null expression [v]. */
    private String ktFromSdk(Shape t, String v, int d) {
        if (t instanceof EnumShape e) return "fromSdk" + enums.get(e.getId()) + "(" + v + ")";
        if (t instanceof StructureShape s) return "fromSdk" + salvoType(s) + "(" + v + ")";
        if (t instanceof ListShape l) {
            return v + ".map { e" + d + " -> " + ktFromSdk(target(l.getMember()), "e" + d, d + 1) + " }";
        }
        if (t instanceof MapShape m) {
            Shape key = target(m.getKey());
            String k = key instanceof EnumShape ? "k" + d + ".value" : "k" + d;
            String sortKey = key instanceof EnumShape ? "it.key.value" : "it.key";
            return v + ".entries.sortedBy { " + sortKey + " }.associate { (k" + d + ", v" + d + ") -> " + k + " to "
                    + ktFromSdk(target(m.getValue()), "v" + d, d + 1) + " }";
        }
        if (t instanceof BlobShape) return "salvo.SalvoBytes(" + v + ")";
        return v;
    }

    private static boolean ktNeedsConv(Shape t) {
        return !(t instanceof StringShape && !(t instanceof EnumShape)) && !isPrimitive(t);
    }

    private String ktReadField(MemberShape m, String src) {
        Shape t = target(m);
        String acc = src + "." + ktMember(m);
        if (present(m)) {
            return ktFromSdk(t, "(" + acc + " ?: error(\"missing required member " + m.getMemberName() + "\"))", 1);
        }
        return ktNeedsConv(t) ? acc + "?.let { " + ktFromSdk(t, "it", 1) + " }" : acc;
    }

    private String ktSetField(MemberShape m, String src) {
        Shape t = target(m);
        String field = src + "." + ktIdent(salvoField(m));
        if (present(m)) return ktToSdk(t, field, 1);
        return ktNeedsConv(t) ? field + "?.let { " + ktToSdk(t, "it", 1) + " }" : field;
    }

    private String kotlinGlue() {
        String effect = settings.effect();
        String pkg = "salvo.platform." + settings.module() + ".host";
        StringBuilder out = new StringBuilder();
        out.append("// Host implementation of `").append(settings.module()).append(".host`'s `Host").append(effect)
                .append("`,\n// over aws-sdk-kotlin (`").append(settings.kotlinPackage()).append("`) [platform-reply].\n//\n");
        out.append(header(""));
        out.append("@file:OptIn(aws.smithy.kotlin.runtime.InternalApi::class, aws.sdk.kotlin.runtime.InternalSdkApi::class)\n");
        out.append("@file:Suppress(\"DEPRECATION\", \"UNUSED_PARAMETER\", \"UNCHECKED_CAST\", \"USELESS_ELVIS\", \"UNNECESSARY_SAFE_CALL\")\n\n");
        out.append("package ").append(pkg).append("\n\nimport salvo.*\nimport salvo.aws.*\nimport salvo.")
                .append(settings.module()).append(".*\nimport salvo.").append(settings.module()).append(".host.*\n");
        out.append("import kotlinx.coroutines.launch\n\n");

        out.append("// `threadsafe platform handler Host").append(effect).append("`: the SDK client is safe to share, and\n")
                .append("// each call is a coroutine on the handler's own scope, completing its reply there.\n");
        out.append("class Host").append(effect).append("(private val config: AwsConfig) : ").append(effect).append(" {\n");
        out.append("    private val scope = kotlinx.coroutines.CoroutineScope(\n")
                .append("        kotlinx.coroutines.SupervisorJob() + kotlinx.coroutines.Dispatchers.IO\n    )\n");
        out.append("    private val client = ").append(ktClient()).append(" {\n")
                .append("        region = this@Host").append(effect).append(".config.region.code\n")
                .append("        credentialsProvider = salvoCredentials(this@Host").append(effect).append(".config.credentials)\n")
                .append("        this@Host").append(effect).append(".config.endpoint?.let { endpointUrl = aws.smithy.kotlin.runtime.net.url.Url.parse(it) }\n")
                .append("    }\n");
        for (OperationShape op : operations) out.append('\n').append(kotlinMember(op));
        out.append("}\n");

        out.append(kotlinCredentials());
        for (ShapeId id : enums.keySet()) out.append('\n').append(kotlinEnumConv(model.expectShape(id, EnumShape.class)));
        for (Map.Entry<ShapeId, String> s : structs.entrySet()) {
            StructureShape shape = model.expectShape(s.getKey(), StructureShape.class);
            if (!inputs.contains(s.getKey())) out.append('\n').append(kotlinFromSdk(shape, s.getValue()));
            if (!outputs.contains(s.getKey())) out.append('\n').append(kotlinToSdk(shape, s.getValue()));
        }
        for (Map.Entry<String, StructureShape> e : errorShapes.entrySet()) {
            out.append('\n').append(kotlinFromSdk(e.getValue(), e.getKey()));
        }
        out.append("""

                private fun salvoAwsError(e: Throwable): AwsError {
                    val code = (e as? aws.smithy.kotlin.runtime.ServiceException)?.sdkErrorMetadata?.errorCode
                        ?: e::class.simpleName ?: "Exception"
                    return AwsError(code = code, message = e.message ?: e.toString())
                }
                """);
        return out.toString();
    }

    private String kotlinMember(OperationShape op) {
        String name = effectOp(op);
        List<String> all = new ArrayList<>(errorNames);
        int n = all.size() + 1;
        StringBuilder out = new StringBuilder();
        out.append("    override fun ").append(name).append('(');
        input(op).ifPresent(in -> out.append("input: ").append(structs.get(in.getId())).append(", "));
        out.append("reply: salvo.SalvoReply) {\n");
        out.append("        val host = reply.hosted()\n");
        String request = input(op).map(in -> "toSdk" + structs.get(in.getId()) + "(input)").orElse("");
        String okType = output(op).map(o -> structs.get(o.getId())).orElse("Unit");
        List<String> errArms = new ArrayList<>(errorNames);
        errArms.add("AwsError");
        String errType = "Union" + errArms.size() + "<" + String.join(", ", errArms) + ">";
        out.append("        scope.launch {\n            val answer: Union2<").append(okType)
                .append(", salvo.core.checked.Checked<").append(errType).append(">> = try {\n");
        String call = "client." + NamingKt.defaultName(op) + "(" + request + ")";
        if (output(op).isPresent()) {
            out.append("                U2_1(fromSdk").append(structs.get(output(op).get().getId())).append('(')
                    .append(call).append("))\n");
        } else {
            out.append("                ").append(call).append("\n                U2_1(Unit)\n");
        }
        for (ShapeId err : op.getErrors(service)) {
            int at = all.indexOf(err.getName()) + 1;
            out.append("            } catch (e: ").append(ktModel()).append('.').append(err.getName()).append(") {\n")
                    .append("                U2_2(salvo.core.checked.Checked(U").append(n).append('_').append(at)
                    .append("(fromSdk").append(err.getName()).append("(e))))\n");
        }
        out.append("            } catch (e: Exception) {\n")
                .append("                U2_2(salvo.core.checked.Checked(U").append(n).append('_').append(n)
                .append("(salvoAwsError(e))))\n            }\n");
        out.append("            host.send(answer)\n        }\n    }\n");
        return out.toString();
    }

    private String kotlinEnumConv(EnumShape e) {
        String name = enums.get(e.getId());
        String sdk = ktModel() + "." + e.getId().getName();
        String union = ktEnumUnion(e);
        int n = e.getEnumValues().size() + 1;
        StringBuilder to = new StringBuilder();
        to.append("private fun toSdk").append(name).append("(v: ").append(union).append("): ").append(sdk)
                .append(" = when (v) {\n");
        StringBuilder from = new StringBuilder();
        from.append("private fun fromSdk").append(name).append("(v: ").append(sdk).append("): ").append(union)
                .append(" = when (v.value) {\n");
        int i = 1;
        for (Map.Entry<String, String> v : e.getEnumValues().entrySet()) {
            to.append("    is U").append(n).append('_').append(i).append(stars(n)).append(" -> ").append(sdk)
                    .append(".fromValue(\"").append(v.getValue()).append("\")\n");
            from.append("    \"").append(v.getValue()).append("\" -> U").append(n).append('_').append(i).append('(')
                    .append(name).append('.').append(pascal(v.getKey())).append("())\n");
            i++;
        }
        to.append("    is U").append(n).append('_').append(n).append(stars(n)).append(" -> ").append(sdk)
                .append(".fromValue((v.value as ").append(name).append('.').append(unknownVariant(e)).append(").value)\n}\n");
        from.append("    else -> U").append(n).append('_').append(n).append('(').append(name).append('.')
                .append(unknownVariant(e)).append("(v.value))\n}\n");
        return to + "\n" + from;
    }

    private String kotlinFromSdk(StructureShape s, String name) {
        StringBuilder out = new StringBuilder();
        String sdk = s.hasTrait("smithy.api#error") ? ktModel() + "." + s.getId().getName() : ktSdkStruct(s.getId());
        out.append("private fun fromSdk").append(name).append("(v: ").append(sdk).append("): ").append(name)
                .append(" = ").append(name).append('(');
        List<MemberShape> members = new ArrayList<>(s.getAllMembers().values());
        if (members.isEmpty()) return out.append(")\n").toString();
        out.append('\n');
        for (MemberShape m : members) {
            out.append("    ").append(ktIdent(salvoField(m))).append(" = ").append(ktReadField(m, "v")).append(",\n");
        }
        out.append(")\n");
        return out.toString();
    }

    private String kotlinToSdk(StructureShape s, String name) {
        String sdk = ktSdkStruct(s.getId());
        StringBuilder out = new StringBuilder();
        out.append("private fun toSdk").append(name).append("(v: ").append(name).append("): ").append(sdk)
                .append(" = ").append(sdk).append(" {\n");
        for (MemberShape m : s.getAllMembers().values()) {
            out.append("    ").append(ktMember(m)).append(" = ").append(ktSetField(m, "v")).append('\n');
        }
        out.append("}\n");
        return out.toString();
    }

    private String kotlinCredentials() {
        return """

                /** The SDK credentials provider an `AwsConfig`'s credentials name. */
                private fun salvoCredentials(
                    c: Union3<ProfileCredentials, EnvironmentCredentials, DefaultChain>,
                ): aws.smithy.kotlin.runtime.auth.awscredentials.CredentialsProvider = when (c) {
                    is U3_1<*, *, *> -> {
                        val p = c.value as ProfileCredentials
                        aws.sdk.kotlin.runtime.auth.credentials.ProfileCredentialsProvider(
                            profileName = p.profile,
                            configurationSource = aws.sdk.kotlin.runtime.config.profile.AwsConfigurationSource(
                                p.profile,
                                salvoExpandHome("~/.aws/config"),
                                salvoExpandHome(p.path),
                            ),
                        )
                    }
                    is U3_2<*, *, *> -> aws.sdk.kotlin.runtime.auth.credentials.EnvironmentCredentialsProvider()
                    is U3_3<*, *, *> -> aws.sdk.kotlin.runtime.auth.credentials.DefaultChainCredentialsProvider()
                }

                /** `~/` at the front of a path is the home directory, as a shell would read it. */
                private fun salvoExpandHome(path: String): String =
                    if (path.startsWith("~/")) (System.getenv("HOME") ?: System.getProperty("user.home")) + path.substring(1) else path
                """;
    }
}
