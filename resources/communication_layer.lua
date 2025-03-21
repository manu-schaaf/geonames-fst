-- Bind static classes from java
StandardCharsets = luajava.bindClass("java.nio.charset.StandardCharsets")
JCasUtil = luajava.bindClass("org.apache.uima.fit.util.JCasUtil")
Token = luajava.bindClass("de.tudarmstadt.ukp.dkpro.core.api.segmentation.type.Token")

location_type = "de.tudarmstadt.ukp.dkpro.core.api.ner.type.Location"


-- This "serialize" function is called to transform the CAS object into an stream that is sent to the annotator
-- Inputs:
--  - inputCas: The actual CAS object to serialize
--  - outputStream: Stream that is sent to the annotator, can be e.g. a string, JSON payload, ...
--  - parameters: Table/Dictonary of parameters that should be used to configure the annotator
function serialize(inputCas, outputStream, parameters)
    local doc_text = inputCas:getDocumentText()
    local doc_lang = inputCas:getDocumentLanguage()

    local Location = luajava.bindClass(parameters["annotation_type"] or location_type)

    local entities = {}
    local nes_it = JCasUtil:select(inputCas, Location):iterator()
    local ne, values = nil, nil
    while nes_it:hasNext() do
        ne = nes_it:next()
        values = {}
        values["begin"] = tostring(ne:getBegin())
        values["end"] = tostring(ne:getEnd())
        values["text"] = ne:getCoveredText()
        table.insert(entities, values)
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
        annotation:setBegin(entity["begin"])
        annotation:setEnd(entity["end"])
        annotation:setId(tonumber(gn["id"]))
        annotation:setName(gn["name"])
        annotation:setLatitude(gn["latitude"])
        annotation:setLongitude(gn["longitude"])
        annotation:setFeatureClass(gn["feature_class"])
        annotation:setFeatureCode(gn["feature_code"])
        annotation:setCountryCode(gn["country_code"])

        local adm = luajava.newInstance("org.apache.uima.jcas.cas.StringArray", inputCas, 4)
        for i, value in ipairs(gn["administrative_divisions"]) do
            adm:set(i - 1, value)
        end
        annotation:setAdm(adm)
        annotation:addToIndexes()
    end
end
