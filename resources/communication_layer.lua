-- Bind static classes from java
StandardCharsets = luajava.bindClass("java.nio.charset.StandardCharsets")
JCasUtil = luajava.bindClass("org.apache.uima.fit.util.JCasUtil")

local annotation_type = "de.tudarmstadt.ukp.dkpro.core.api.ner.type.Location"
local references = {}
local headers = {
    ["Content-Type"] = "application/json"
}

-- This "serialize" function is called to transform the CAS object into an stream that is sent to the annotator
-- Inputs:
--  - inputCas: The actual CAS object to serialize
--  - outputStream: Stream that is sent to the annotator, can be e.g. a string, JSON payload, ...
--  - parameters: Table/Dictonary of parameters that should be used to configure the annotator
function serialize(inputCas, outputStream, parameters)
    references = {}
    if parameters ~= nil and parameters["annotation_type"] ~= nil then
        annotation_type = parameters["annotation_type"]
    end

    local nes_it = JCasUtil:select(inputCas, luajava.bindClass(annotation_type)):iterator()
    local entities = {}
    while nes_it:hasNext() do
        local ne = nes_it:next()
        table.insert(references, ne)

        table.insert(entities, {
            ["reference"] = tostring(#references),
            ["text"] = ne:getCoveredText()
        })
    end

    local query = {
        queries = entities,
        mode = "find",
        result_selection = "first",
    }
    if parameters ~= nil then
        local mode = "find"
        if parameters["mode"] ~= nil then
            mode = parameters["mode"]
        end
        query["mode"] = mode

        if parameters["filter"] ~= nil then
            query["filter"] = parameters["filter"]
        end

        if mode ~= "find" then
            query["max_dist"] = tostring(parameters["max_dist"])
        end

        if mode == "levenshtein" and parameters["state_limit"] ~= nil then
            query["state_limit"] = tostring(parameters["state_limit"])
        end

        if parameters["result_selection"] ~= nil then
            query["result_selection"] = parameters["result_selection"]
        end
    end

    outputStream:write(json.encode(query))

    return {
        headers = headers,
    }
end

-- This "deserialize" function is called on receiving the results from the annotator that have to be transformed into a CAS object
-- Inputs:
--  - inputCas: The actual CAS object to deserialize into
--  - inputStream: Stream that is received from to the annotator, can be e.g. a string, JSON payload, ...
function deserialize(inputCas, inputStream)
    -- Get string from stream, assume UTF-8 encoding
    local inputString = luajava.newInstance("java.lang.String", inputStream:readAllBytes(), StandardCharsets.UTF_8)

    -- Parse JSON data from string into object
    local results = json.decode(inputString)

    local results_modification = results["modification"]
    local document_modification = luajava.newInstance("org.texttechnologylab.annotation.DocumentModification", inputCas)
    document_modification:setUser(results_modification["user"])
    document_modification:setTimestamp(results_modification["timestamp"])
    document_modification:setComment(results_modification["comment"])
    document_modification:addToIndexes()

    local gn, annotation = nil, nil
    for _, entity in ipairs(results["results"]) do
        gn = entity["entry"]

        -- TODO: Requires UIMATypeSystem version >= 3.0.6
        annotation = luajava.newInstance("org.texttechnologylab.annotation.geonames.GeoNamesEntity", inputCas)
        annotation:setId(tonumber(gn["id"]))
        annotation:setName(gn["name"])
        annotation:setLatitude(gn["latitude"])
        annotation:setLongitude(gn["longitude"])
        annotation:setFeatureClass(gn["feature_class"])
        annotation:setFeatureCode(gn["feature_code"])
        annotation:setCountryCode(gn["country_code"])
        annotation:setAdm1(gn["adm1"])
        annotation:setAdm2(gn["adm2"])
        annotation:setAdm3(gn["adm3"])
        annotation:setAdm4(gn["adm4"])

        if gn["elevation"] ~= nil then
            annotation:setElevation(gn["elevation"])
        end

        local reference = references[tonumber(entity["reference"])]
        if reference == nil then
            error("Failed to resolve reference annotation with index " .. entity["reference"])
        else
            annotation:setReferenceAnnotation(reference)
            annotation:setBegin(reference:getBegin())
            annotation:setEnd(reference:getEnd())
        end

        annotation:addToIndexes()
    end
end
